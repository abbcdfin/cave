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
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UserConfigData {
    #[serde(default)]
    pub defaults: Option<UserDefaultsConfig>,
    #[serde(default)]
    pub profiles: HashMap<String, UserAgentProfileConfig>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UserDefaultsConfig {
    #[serde(default)]
    pub ro_system: Vec<String>,
    #[serde(default)]
    pub ro_home: Vec<String>,
    #[serde(default)]
    pub rw_cache: Vec<String>,
    #[serde(default)]
    pub tmpfs: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UserAgentProfileConfig {
    #[serde(default)]
    pub state: Vec<StateEntry>,
    #[serde(default)]
    pub runtime: Vec<String>,
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
    let mut config: ConfigData = toml::from_str(PROFILES_TOML).expect("Failed to parse embedded profiles.toml");
    config.sources = vec!["embedded profiles.toml".to_string()];
    config
}

pub fn user_config_path(home_dir: &Path) -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let is_matching_home = std::env::var_os("HOME").map(PathBuf::from).as_deref() == Some(home_dir);
        if !xdg.is_empty() && (is_matching_home || Path::new(&xdg).starts_with(home_dir)) {
            return PathBuf::from(xdg).join("cave").join("profiles.toml");
        }
    }
    home_dir.join(".config").join("cave").join("profiles.toml")
}

pub fn load_effective_config(home_dir: &Path) -> Result<ConfigData, String> {
    let mut config = load_config();
    let config_path = user_config_path(home_dir);

    if !config_path.exists() {
        return Ok(config);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Error reading user config file '{}': {}", config_path.display(), e))?;

    let user_config: UserConfigData = toml::from_str(&content)
        .map_err(|e| format!("Error parsing user config file '{}': {}", config_path.display(), e))?;

    merge_user_config(&mut config, user_config);
    config.sources.push(config_path.display().to_string());

    Ok(config)
}

pub fn merge_user_config(config: &mut ConfigData, user_config: UserConfigData) {
    if let Some(user_defaults) = user_config.defaults {
        for s in user_defaults.ro_system {
            if !config.defaults.ro_system.contains(&s) {
                config.defaults.ro_system.push(s);
            }
        }
        for s in user_defaults.tmpfs {
            if !config.defaults.tmpfs.contains(&s) {
                config.defaults.tmpfs.push(s);
            }
        }
        for rel in user_defaults.ro_home {
            // Downgrade: if present in rw_cache, remove from rw_cache
            config.defaults.rw_cache.retain(|x| x != &rel);
            if !config.defaults.ro_home.contains(&rel) {
                config.defaults.ro_home.push(rel);
            }
        }
        for rel in user_defaults.rw_cache {
            // If present in ro_home, remove from ro_home
            config.defaults.ro_home.retain(|x| x != &rel);
            if !config.defaults.rw_cache.contains(&rel) {
                config.defaults.rw_cache.push(rel);
            }
        }
    }

    for (name, user_prof) in user_config.profiles {
        let entry = config.profiles.entry(name).or_insert_with(|| AgentProfileConfig {
            state: Vec::new(),
            runtime: Vec::new(),
        });
        for s in user_prof.state {
            entry.runtime.retain(|x| x != &s.path);
            if !entry.state.iter().any(|x| x.path == s.path) {
                entry.state.push(s);
            }
        }
        for r in user_prof.runtime {
            // Downgrade: if present in state, remove from state
            entry.state.retain(|x| x.path != r);
            if !entry.runtime.contains(&r) {
                entry.runtime.push(r);
            }
        }
    }
}

pub fn agent_basename(agent_cmd: &OsStr) -> &str {
    Path::new(agent_cmd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
}

/// Explicit host side-effect step: ensures declared state files/directories exist
/// before mounting so they can persist, seeding files with valid initial content.
pub fn prepare_host_state(agent_cmd: &OsStr, home_dir: &Path, config: &ConfigData) -> std::io::Result<()> {
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

/// Pure policy function: builds ordered mount plan without mutating the host filesystem or reading disk.
pub fn build_mount_plan(
    options: &RunOptions,
    home_dir: &Path,
    cwd: &Path,
    uid: u32,
    config: &ConfigData,
) -> MountPlan {
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
        // Runtime paths (Read-Only) - rendered after state so an RO subpath inside an RW state dir overrides it
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
        assert!(config.profiles.contains_key("agy"));
        assert!(config.profiles.contains_key("aider"));
        assert!(config.profiles.contains_key("sh"));
        assert!(config.profiles.contains_key("bash"));
    }

    #[test]
    fn test_prepare_host_state_seeds_valid_json() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(&home).unwrap();

        // Initially absent
        assert!(!home.join(".claude").exists());
        let config = load_config();
        prepare_host_state(OsStr::new("claude"), &home, &config).unwrap();

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

        let config = load_config();
        // Calling build_mount_plan does not create missing files on host
        let _plan = build_mount_plan(&opts, &home, &cwd, 1000, &config);
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

        let config = load_config();
        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config);

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

    #[test]
    fn test_merge_user_config_append() {
        let mut config = load_config();
        let user_toml = r#"
            [defaults]
            ro_system = ["/opt/custom"]
            ro_home = [".custom_tool"]
            rw_cache = [".custom_cache"]
            tmpfs = ["/custom_tmp"]

            [profiles.claude]
            runtime = [".local/share/claude-extra"]

            [profiles.mybot]
            runtime = [".mybot/bin"]
            [[profiles.mybot.state]]
            path = ".mybot/data"
            kind = "dir"
        "#;
        let user_cfg: UserConfigData = toml::from_str(user_toml).unwrap();
        merge_user_config(&mut config, user_cfg);

        assert!(config.defaults.ro_system.contains(&"/opt/custom".to_string()));
        assert!(config.defaults.ro_home.contains(&".custom_tool".to_string()));
        assert!(config.defaults.rw_cache.contains(&".custom_cache".to_string()));
        assert!(config.defaults.tmpfs.contains(&"/custom_tmp".to_string()));

        let claude = config.profiles.get("claude").unwrap();
        assert!(claude.runtime.contains(&".local/share/claude-extra".to_string()));

        let mybot = config.profiles.get("mybot").unwrap();
        assert!(mybot.runtime.contains(&".mybot/bin".to_string()));
        assert!(mybot.state.iter().any(|s| s.path == ".mybot/data"));
    }

    #[test]
    fn test_merge_user_config_downgrade_by_redeclaration() {
        let mut config = load_config();
        assert!(config.defaults.rw_cache.contains(&".cache".to_string()));

        let user_toml = r#"
            [defaults]
            ro_home = [".cache"]
        "#;
        let user_cfg: UserConfigData = toml::from_str(user_toml).unwrap();
        merge_user_config(&mut config, user_cfg);

        assert!(config.defaults.ro_home.contains(&".cache".to_string()));
        assert!(!config.defaults.rw_cache.contains(&".cache".to_string()));
    }

    #[test]
    fn test_load_effective_config_malformed_has_location() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let cfg_dir = home.join(".config").join("cave");
        fs::create_dir_all(&cfg_dir).unwrap();
        fs::write(cfg_dir.join("profiles.toml"), "invalid [ toml syntax :::").unwrap();

        let res = load_effective_config(home);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Error parsing user config file"));
        assert!(err.contains("line"));
    }

    #[test]
    fn test_load_effective_config_absent_ok() {
        let dir = tempdir().unwrap();
        let res = load_effective_config(dir.path());
        assert!(res.is_ok());
        let cfg = res.unwrap();
        assert!(cfg.profiles.contains_key("claude"));
    }

    #[test]
    fn test_profile_mount_order_state_before_runtime() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        let state_dir = home.join(".test_agent");
        let runtime_subpath = state_dir.join("bin");
        fs::create_dir_all(&runtime_subpath).unwrap();

        let cfg_dir = home.join(".config").join("cave");
        fs::create_dir_all(&cfg_dir).unwrap();
        let user_toml = r#"
            [profiles.test_agent]
            runtime = [".test_agent/bin"]
            [[profiles.test_agent.state]]
            path = ".test_agent"
            kind = "dir"
        "#;
        fs::write(cfg_dir.join("profiles.toml"), user_toml).unwrap();

        let cwd = PathBuf::from("/");
        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("test_agent"),
            agent_args: vec![],
        };

        let config = load_effective_config(home).expect("effective config loads");
        let plan = build_mount_plan(&opts, home, &cwd, 1000, &config);

        let mut state_idx = None;
        let mut runtime_idx = None;

        for (idx, mount) in plan.mounts.iter().enumerate() {
            if mount.target_path == state_dir && mount.kind == MountKind::BindReadWrite {
                state_idx = Some(idx);
            }
            if mount.target_path == runtime_subpath && mount.kind == MountKind::BindReadOnly {
                runtime_idx = Some(idx);
            }
        }

        assert!(state_idx.is_some(), "State mount not found");
        assert!(runtime_idx.is_some(), "Runtime mount not found");
        assert!(
            state_idx.unwrap() < runtime_idx.unwrap(),
            "State (RW) must be mounted before Runtime (RO) so subpaths can be read-only"
        );
    }
}
