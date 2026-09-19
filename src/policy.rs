use crate::cli::RunOptions;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountKind {
    BindReadWrite,
    BindReadOnly,
    Tmpfs,
    Proc,
    Dev,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountEntry {
    pub host_path: Option<PathBuf>,
    pub target_path: PathBuf,
    pub kind: MountKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountPlan {
    pub mounts: Vec<MountEntry>,
    pub set_env: Vec<(String, String)>,
    pub unset_env: Vec<String>,
    pub workdir: PathBuf,
    pub uid: u32,
}

#[derive(Debug, Deserialize)]
pub struct ConfigData {
    pub defaults: DefaultsConfig,
    pub profiles: HashMap<String, AgentProfileConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    pub ro_system: Vec<String>,
    pub ro_home: Vec<String>,
    pub rw_cache: Vec<String>,
    pub tmpfs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentProfileConfig {
    pub state: Vec<String>,
    pub runtime: Vec<String>,
}

pub static PROFILES_TOML: &str = include_str!("profiles.toml");

pub fn load_config() -> ConfigData {
    toml::from_str(PROFILES_TOML).expect("Failed to parse embedded profiles.toml")
}

pub fn build_mount_plan(
    options: &RunOptions,
    home_dir: &Path,
    cwd: &Path,
    uid: u32,
) -> MountPlan {
    let config = load_config();
    let mut mounts = Vec::new();

    // 1. Kernel filesystems
    mounts.push(MountEntry {
        host_path: None,
        target_path: PathBuf::from("/proc"),
        kind: MountKind::Proc,
    });
    mounts.push(MountEntry {
        host_path: None,
        target_path: PathBuf::from("/dev"),
        kind: MountKind::Dev,
    });

    // 2. Read-only system paths
    for s in &config.defaults.ro_system {
        let p = PathBuf::from(s);
        if p.exists() {
            mounts.push(MountEntry {
                host_path: Some(p.clone()),
                target_path: p,
                kind: MountKind::BindReadOnly,
            });
        }
    }

    // 3. Private tmpfs mounts (/tmp, /run, /var/tmp)
    for s in &config.defaults.tmpfs {
        mounts.push(MountEntry {
            host_path: None,
            target_path: PathBuf::from(s),
            kind: MountKind::Tmpfs,
        });
    }

    // 4. DNS: bind the file /etc/resolv.conf points to, keeping /run private
    let resolv_conf = Path::new("/etc/resolv.conf");
    if resolv_conf.exists() {
        let resolved = fs::canonicalize(resolv_conf).unwrap_or_else(|_| resolv_conf.to_path_buf());
        mounts.push(MountEntry {
            host_path: Some(resolved),
            target_path: resolv_conf.to_path_buf(),
            kind: MountKind::BindReadOnly,
        });
    }

    // 5. Private $HOME tmpfs
    mounts.push(MountEntry {
        host_path: None,
        target_path: home_dir.to_path_buf(),
        kind: MountKind::Tmpfs,
    });

    // 6. Default toolchains in $HOME (Read-Only)
    for rel in &config.defaults.ro_home {
        let p = home_dir.join(rel);
        if p.exists() {
            mounts.push(MountEntry {
                host_path: Some(p.clone()),
                target_path: p,
                kind: MountKind::BindReadOnly,
            });
        }
    }

    // 7. Default shared tool caches in $HOME (Read-Write)
    for rel in &config.defaults.rw_cache {
        let p = home_dir.join(rel);
        if p.exists() {
            mounts.push(MountEntry {
                host_path: Some(p.clone()),
                target_path: p,
                kind: MountKind::BindReadWrite,
            });
        }
    }

    // 8. Agent Profile
    let agent_name = Path::new(&options.agent_cmd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&options.agent_cmd);

    if let Some(profile) = config.profiles.get(agent_name) {
        // Runtime paths (Read-Only)
        for rel in &profile.runtime {
            let p = home_dir.join(rel);
            if p.exists() {
                mounts.push(MountEntry {
                    host_path: Some(p.clone()),
                    target_path: p,
                    kind: MountKind::BindReadOnly,
                });
            }
        }
        // State paths (Read-Write)
        for rel in &profile.state {
            let p = home_dir.join(rel);
            if p.exists() {
                mounts.push(MountEntry {
                    host_path: Some(p.clone()),
                    target_path: p,
                    kind: MountKind::BindReadWrite,
                });
            }
        }
    }

    // 9. Current working directory (Read-Write at host path)
    let canonical_cwd = fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    mounts.push(MountEntry {
        host_path: Some(canonical_cwd.clone()),
        target_path: canonical_cwd.clone(),
        kind: MountKind::BindReadWrite,
    });

    // 10. Command-line overrides (--dir / --ro-dir)
    for path in &options.dirs {
        let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        mounts.push(MountEntry {
            host_path: Some(canonical_path.clone()),
            target_path: canonical_path,
            kind: MountKind::BindReadWrite,
        });
    }

    for path in &options.ro_dirs {
        let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        mounts.push(MountEntry {
            host_path: Some(canonical_path.clone()),
            target_path: canonical_path,
            kind: MountKind::BindReadOnly,
        });
    }

    // Environment plan
    let mut set_env = Vec::new();
    set_env.push(("HOME".to_string(), home_dir.display().to_string()));
    set_env.push(("XDG_RUNTIME_DIR".to_string(), format!("/run/user/{uid}")));

    let unset_env = vec![
        "SSH_AUTH_SOCK".to_string(),
        "DBUS_SESSION_BUS_ADDRESS".to_string(),
    ];

    MountPlan {
        mounts,
        set_env,
        unset_env,
        workdir: canonical_cwd,
        uid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_config() {
        let config = load_config();
        assert!(config.defaults.ro_system.contains(&"/usr".to_string()));
        assert!(config.defaults.tmpfs.contains(&"/tmp".to_string()));
        assert!(config.profiles.contains_key("claude"));
    }

    #[test]
    fn test_build_mount_plan_claude() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        // Create claude state and runtime in home
        fs::create_dir_all(home.join(".claude")).unwrap();
        fs::write(home.join(".claude.json"), "{}").unwrap();
        fs::create_dir_all(home.join(".local/share/claude")).unwrap();

        let opts = RunOptions {
            dirs: vec![],
            ro_dirs: vec![],
            agent_cmd: "claude".to_string(),
            agent_args: vec![],
        };

        let plan = build_mount_plan(&opts, &home, &cwd, 1000);

        // Check that ~/.claude is ReadWrite
        let claude_mount = plan
            .mounts
            .iter()
            .find(|m| m.target_path == home.join(".claude"))
            .expect("~/.claude should be mounted");
        assert_eq!(claude_mount.kind, MountKind::BindReadWrite);

        // Check that ~/.local/share/claude is ReadOnly
        let runtime_mount = plan
            .mounts
            .iter()
            .find(|m| m.target_path == home.join(".local/share/claude"))
            .expect("runtime should be mounted");
        assert_eq!(runtime_mount.kind, MountKind::BindReadOnly);

        // Check CWD is mounted ReadWrite
        let canonical_cwd = fs::canonicalize(&cwd).unwrap();
        let cwd_mount = plan
            .mounts
            .iter()
            .find(|m| m.target_path == canonical_cwd)
            .expect("CWD should be mounted");
        assert_eq!(cwd_mount.kind, MountKind::BindReadWrite);

        // Check environment plan
        assert!(plan.unset_env.contains(&"SSH_AUTH_SOCK".to_string()));
        assert!(plan.unset_env.contains(&"DBUS_SESSION_BUS_ADDRESS".to_string()));
        assert!(plan
            .set_env
            .iter()
            .any(|(k, v)| k == "XDG_RUNTIME_DIR" && v == "/run/user/1000"));
    }

    #[test]
    fn test_mount_order_overrides() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            dirs: vec![PathBuf::from("/tmp")],
            ro_dirs: vec![cwd.clone()],
            agent_cmd: "sh".to_string(),
            agent_args: vec![],
        };

        let plan = build_mount_plan(&opts, &home, &cwd, 1000);

        // Verify that the user's /tmp mount comes AFTER the initial tmpfs /tmp
        let tmp_indices: Vec<usize> = plan
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.target_path == PathBuf::from("/tmp"))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(tmp_indices.len(), 2);
        assert_eq!(plan.mounts[tmp_indices[0]].kind, MountKind::Tmpfs);
        assert_eq!(plan.mounts[tmp_indices[1]].kind, MountKind::BindReadWrite);

        // Verify that the user's --ro-dir for cwd comes AFTER the default cwd mount
        let canonical_cwd = fs::canonicalize(&cwd).unwrap();
        let cwd_indices: Vec<usize> = plan
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.target_path == canonical_cwd)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(cwd_indices.len(), 2);
        assert_eq!(plan.mounts[cwd_indices[0]].kind, MountKind::BindReadWrite);
        assert_eq!(plan.mounts[cwd_indices[1]].kind, MountKind::BindReadOnly);
    }
}
