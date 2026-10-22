use crate::cli::{ExtraDir, RunOptions};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
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

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigData {
    pub defaults: DefaultsConfig,
    pub profiles: HashMap<String, AgentProfileConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DefaultsConfig {
    pub ro_system: Vec<String>,
    pub ro_home: Vec<String>,
    pub rw_cache: Vec<String>,
    pub tmpfs: Vec<String>,
}

/// Declared host state entry for an agent.
///
/// NOTE on single-file bind mounts vs kernel rename:
/// When a single file (e.g. `~/.claude.json`) is bind-mounted directly, Linux VFS treats
/// that path inside the mount namespace as an active mount point. Any process inside the
/// sandbox attempting an atomic file replacement via `rename(temp, target)` will receive
/// `EBUSY`. Claude Code updates config in-place (`open(O_WRONLY|O_TRUNC)`), which succeeds
/// on the existing inode. To prevent a 0-byte corrupt file on first run, `init` seeds
/// newly-created state files with valid initial content (e.g. `"{}"`).
#[derive(Debug, Deserialize, Clone)]
pub struct StateEntry {
    pub path: String,
    pub kind: String,
    pub init: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentProfileConfig {
    pub state: Vec<StateEntry>,
    pub runtime: Vec<String>,
}

pub static PROFILES_TOML: &str = include_str!("profiles.toml");

pub fn load_config() -> ConfigData {
    toml::from_str(PROFILES_TOML).expect("Failed to parse embedded profiles.toml")
}

pub fn agent_basename(agent_cmd: &OsStr) -> &str {
    Path::new(agent_cmd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
}

/// Explicit host side-effect step: ensures declared state files/directories exist
/// before mounting so they can persist, seeding files with valid initial content.
pub fn prepare_host_state(agent_cmd: &OsStr, home_dir: &Path) -> std::io::Result<()> {
    let config = load_config();
    let name = agent_basename(agent_cmd);

    if let Some(profile) = config.profiles.get(name) {
        for entry in &profile.state {
            let target = home_dir.join(&entry.path);
            if !target.exists() {
                if entry.kind == "dir" {
                    fs::create_dir_all(&target)?;
                } else if entry.kind == "file" {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target, entry.init.as_deref().unwrap_or(""))?;
                }
            }
        }
    }
    Ok(())
}

/// Pure policy function: builds ordered mount plan without mutating the host filesystem.
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
    let name = agent_basename(&options.agent_cmd);
    if let Some(profile) = config.profiles.get(name) {
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
        for entry in &profile.state {
            let p = home_dir.join(&entry.path);
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
    let canonical_home = fs::canonicalize(home_dir).unwrap_or_else(|_| home_dir.to_path_buf());

    if canonical_cwd == canonical_home {
        eprintln!(
            "Warning: current working directory is {}, which exposes the entire host home directory, defeating sandbox boundaries.",
            canonical_cwd.display()
        );
    } else if canonical_cwd == Path::new("/") {
        eprintln!(
            "Warning: current working directory is /, which exposes the host root filesystem, defeating sandbox boundaries."
        );
    }

    mounts.push(MountEntry {
        host_path: Some(canonical_cwd.clone()),
        target_path: canonical_cwd.clone(),
        kind: MountKind::BindReadWrite,
    });

    // 10. Command-line overrides (--dir / --ro-dir in exact specified order)
    for extra in &options.extra_dirs {
        match extra {
            ExtraDir::ReadWrite(path) => {
                let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                mounts.push(MountEntry {
                    host_path: Some(canonical_path.clone()),
                    target_path: canonical_path,
                    kind: MountKind::BindReadWrite,
                });
            }
            ExtraDir::ReadOnly(path) => {
                let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                mounts.push(MountEntry {
                    host_path: Some(canonical_path.clone()),
                    target_path: canonical_path,
                    kind: MountKind::BindReadOnly,
                });
            }
        }
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
    use std::ffi::OsString;
    use tempfile::tempdir;

    #[test]
    fn test_load_config() {
        let config = load_config();
        assert!(config.defaults.ro_system.contains(&"/usr".to_string()));
        assert!(config.defaults.tmpfs.contains(&"/tmp".to_string()));
        assert!(config.profiles.contains_key("claude"));
    }

    #[test]
    fn test_prepare_host_state_seeds_valid_json() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(&home).unwrap();

        // Initially absent
        assert!(!home.join(".claude").exists());
        assert!(!home.join(".claude.json").exists());

        prepare_host_state(OsStr::new("claude"), &home).unwrap();

        // .claude is created as dir
        assert!(home.join(".claude").is_dir());
        // .claude.json is created as file and contains "{}"
        assert!(home.join(".claude.json").is_file());
        let content = fs::read_to_string(home.join(".claude.json")).unwrap();
        assert_eq!(content, "{}");
    }

    #[test]
    fn test_build_mount_plan_is_pure() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("claude"),
            agent_args: vec![],
        };

        // Calling build_mount_plan does not create missing files on host
        let _plan = build_mount_plan(&opts, &home, &cwd, 1000);
        assert!(!home.join(".claude").exists());
        assert!(!home.join(".claude.json").exists());
    }

    #[test]
    fn test_command_line_mount_order_respected() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        let sub = cwd.join("sub");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&sub).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![
                ExtraDir::ReadOnly(cwd.clone()),
                ExtraDir::ReadWrite(sub.clone()),
            ],
            agent_cmd: OsString::from("sh"),
            agent_args: vec![],
        };

        let plan = build_mount_plan(&opts, &home, &cwd, 1000);

        let canonical_cwd = fs::canonicalize(&cwd).unwrap();
        let canonical_sub = fs::canonicalize(&sub).unwrap();

        let cwd_ro_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == canonical_cwd && m.kind == MountKind::BindReadOnly)
            .expect("cwd RO mount should exist");

        let sub_rw_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == canonical_sub && m.kind == MountKind::BindReadWrite)
            .expect("sub RW mount should exist");

        assert!(
            sub_rw_idx > cwd_ro_idx,
            "sub RW mount ({sub_rw_idx}) must come after cwd RO mount ({cwd_ro_idx})"
        );
    }
}
