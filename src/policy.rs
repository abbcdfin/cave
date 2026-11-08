use crate::cli::{ExtraDir, RunOptions};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct TcpCheckConfig {
    pub addr: String,
    pub hint: String,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ToolchainConfig {
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub rw_cache: Vec<String>,
    #[serde(default)]
    pub runtime: Vec<String>,
    #[serde(default)]
    pub check_tcp: Vec<TcpCheckConfig>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct UserToolchainConfig {
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub rw_cache: Vec<String>,
    #[serde(default)]
    pub runtime: Vec<String>,
    #[serde(default)]
    pub check_tcp: Vec<TcpCheckConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToolchain {
    pub name: String,
    pub rw_cache: Vec<PathBuf>,
    pub runtime: Vec<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigData {
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub profiles: HashMap<String, AgentProfileConfig>,
    #[serde(default)]
    pub toolchains: HashMap<String, ToolchainConfig>,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UserConfigData {
    #[serde(default)]
    pub defaults: Option<UserDefaultsConfig>,
    #[serde(default)]
    pub profiles: HashMap<String, UserAgentProfileConfig>,
    #[serde(default)]
    pub toolchains: HashMap<String, UserToolchainConfig>,
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

    for (name, user_tc) in user_config.toolchains {
        let entry = config.toolchains.entry(name).or_default();
        for req in user_tc.requires {
            if !entry.requires.contains(&req) {
                entry.requires.push(req);
            }
        }
        for rw in user_tc.rw_cache {
            // Move: if present in runtime, remove from runtime
            entry.runtime.retain(|x| x != &rw);
            if !entry.rw_cache.contains(&rw) {
                entry.rw_cache.push(rw);
            }
        }
        for ro in user_tc.runtime {
            // Move: if present in rw_cache, remove from rw_cache
            entry.rw_cache.retain(|x| x != &ro);
            if !entry.runtime.contains(&ro) {
                entry.runtime.push(ro);
            }
        }
        for tcp in user_tc.check_tcp {
            if !entry.check_tcp.iter().any(|x| x.addr == tcp.addr) {
                entry.check_tcp.push(tcp);
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

pub fn is_socket(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        fs::metadata(path)
            .map(|m| m.file_type().is_socket())
            .unwrap_or(false)
        }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn check_tcp_port(addr_str: &str, timeout_ms: u64) -> bool {
    let addrs: Vec<SocketAddr> = if let Ok(addr) = addr_str.parse::<SocketAddr>() {
        vec![addr]
    } else if let Ok(iter) = addr_str.to_socket_addrs() {
        iter.collect()
    } else {
        return false;
    };

    let timeout = Duration::from_millis(timeout_ms);
    for addr in addrs {
        if TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return true;
        }
    }
    false
}

#[derive(Debug, PartialEq, Eq)]
pub enum SshSocket {
    Unset,
    Stale(PathBuf),
    Valid(PathBuf),
}

pub fn resolve_ssh_socket() -> SshSocket {
    match std::env::var_os("SSH_AUTH_SOCK") {
        None => SshSocket::Unset,
        Some(val) if val.is_empty() => SshSocket::Unset,
        Some(val) => {
            let path = PathBuf::from(val);
            if path.exists() && is_socket(&path) {
                SshSocket::Valid(path)
            } else {
                SshSocket::Stale(path)
            }
        }
    }
}

pub fn normalize_path_lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = components.last() {
                    components.pop();
                } else if !components.iter().any(|c| matches!(c, Component::RootDir | Component::Prefix(_))) {
                    components.push(comp);
                }
            }
            _ => components.push(comp),
        }
    }
    components.into_iter().collect()
}

pub trait ExpansionLookup {
    fn lookup_var(&self, var: &str) -> Option<String>;
    fn lookup_bin(&self, bin: &str) -> Option<PathBuf>;
}

pub struct SystemLookup;

impl ExpansionLookup for SystemLookup {
    fn lookup_var(&self, var: &str) -> Option<String> {
        std::env::var(var).ok().filter(|v| !v.is_empty())
    }

    fn lookup_bin(&self, bin: &str) -> Option<PathBuf> {
        if let Some(p) = crate::bwrap::which_binary(bin) {
            fs::canonicalize(&p).ok().or(Some(p))
        } else {
            None
        }
    }
}

pub fn expand_string<L: ExpansionLookup + ?Sized>(template: &str, lookup: &L) -> Option<String> {
    let mut result = String::new();
    let mut rest = template;

    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let end = after_start.find('}')?;
        let inner = &after_start[..end];
        rest = &after_start[end + 1..];

        if let Some(bin_name) = inner.strip_prefix("bin:") {
            let bin_path = lookup.lookup_bin(bin_name)?;
            result.push_str(&bin_path.to_string_lossy());
        } else if let Some((var, default_val)) = inner.split_once(":-") {
            let val = lookup
                .lookup_var(var)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| default_val.to_string());
            result.push_str(&val);
        } else {
            let val = lookup.lookup_var(inner).filter(|v| !v.is_empty())?;
            result.push_str(&val);
        }
    }
    result.push_str(rest);
    Some(result)
}

pub fn expand_entry<L: ExpansionLookup + ?Sized>(
    entry: &str,
    home: &Path,
    lookup: &L,
) -> Option<PathBuf> {
    let expanded = expand_string(entry, lookup)?;
    let path = if expanded.starts_with('/') {
        PathBuf::from(expanded)
    } else {
        home.join(expanded)
    };
    Some(normalize_path_lexical(&path))
}

pub fn resolve_path_literal(raw: &str, home: &Path) -> PathBuf {
    let p = if raw.starts_with('/') {
        PathBuf::from(raw)
    } else {
        home.join(raw)
    };
    normalize_path_lexical(&p)
}

pub fn resolve_literal_toolchain(name: &str, tc: &ToolchainConfig, home: &Path) -> ResolvedToolchain {
    let rw_cache = tc.rw_cache.iter().map(|p| resolve_path_literal(p, home)).collect();
    let runtime = tc.runtime.iter().map(|p| resolve_path_literal(p, home)).collect();
    ResolvedToolchain {
        name: name.to_string(),
        rw_cache,
        runtime,
    }
}

/// Pure policy function: builds ordered mount plan without mutating the host filesystem or reading environment/config.
pub fn build_mount_plan(
    options: &RunOptions,
    home_dir: &Path,
    cwd: &Path,
    uid: u32,
    config: &ConfigData,
    ssh_socket: Option<&Path>,
    toolchains: &[ResolvedToolchain],
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

    // 3. Private tmpfs mounts (/tmp, /run, /var/tmp, /etc/ssh/ssh_config.d)
    for s in &config.defaults.tmpfs {
        let p = PathBuf::from(s);
        if p.exists() {
            mounts.push(MountEntry {
                host_path: None,
                target_path: p,
                kind: MountKind::Tmpfs,
            });
        }
    }

    // 3a. Forward host SSH agent socket if available into private runtime dir
    if let Some(sock) = ssh_socket {
        mounts.push(MountEntry {
            host_path: Some(sock.to_path_buf()),
            target_path: PathBuf::from(format!("/run/user/{uid}/ssh-agent.sock")),
            kind: MountKind::BindReadOnly,
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

    // 8b. Toolchains (in --with order)
    for tc in toolchains {
        // rw_cache (Read-Write) rendered before runtime
        for p in &tc.rw_cache {
            if p.exists() {
                mounts.push(MountEntry {
                    host_path: Some(p.clone()),
                    target_path: p.clone(),
                    kind: MountKind::BindReadWrite,
                });
            }
        }
        // runtime (Read-Only)
        for p in &tc.runtime {
            if p.exists() {
                mounts.push(MountEntry {
                    host_path: Some(p.clone()),
                    target_path: p.clone(),
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

    let mut unset_env = vec![
        "DBUS_SESSION_BUS_ADDRESS".to_string(),
    ];

    if ssh_socket.is_some() {
        set_env.push(("SSH_AUTH_SOCK".to_string(), format!("/run/user/{uid}/ssh-agent.sock")));
    } else {
        unset_env.push("SSH_AUTH_SOCK".to_string());
    }

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
            no_ssh: false,
            with_toolchains: vec![],
        };

        let config = load_config();
        // Calling build_mount_plan does not create missing files on host
        let _plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, None, &[]);
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
            no_ssh: false,
            with_toolchains: vec![],
        };

        let config = load_config();
        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, None, &[]);

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
            no_ssh: false,
            with_toolchains: vec![],
        };

        let config = load_effective_config(home).expect("effective config loads");
        let plan = build_mount_plan(&opts, home, &cwd, 1000, &config, None, &[]);

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

    #[test]
    fn test_ssh_defaults_in_profiles_toml() {
        let config = load_config();
        assert!(config.defaults.ro_home.contains(&".ssh/config".to_string()));
        assert!(config.defaults.ro_home.contains(&".ssh/known_hosts".to_string()));
    }

    #[test]
    fn test_build_mount_plan_with_ssh_socket() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        let sock = dir.path().join("fake_agent.sock");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("claude"),
            agent_args: vec![],
            no_ssh: false,
            with_toolchains: vec![],
        };

        let config = load_config();
        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, Some(&sock), &[]);

        // 1. Target mount at /run/user/1000/ssh-agent.sock exists with BindReadOnly
        let target_sock = PathBuf::from("/run/user/1000/ssh-agent.sock");
        let sock_mount_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == target_sock && m.kind == MountKind::BindReadOnly)
            .expect("SSH socket bind mount must exist");

        assert_eq!(plan.mounts[sock_mount_idx].host_path, Some(sock.clone()));

        // 2. Mounted after /run tmpfs
        let run_tmpfs_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == Path::new("/run") && m.kind == MountKind::Tmpfs)
            .expect("/run tmpfs must exist");
        assert!(
            sock_mount_idx > run_tmpfs_idx,
            "SSH socket mount ({sock_mount_idx}) must come after /run tmpfs ({run_tmpfs_idx})"
        );

        // 3. Environment: SSH_AUTH_SOCK is in set_env and not in unset_env
        assert!(plan.set_env.contains(&(
            "SSH_AUTH_SOCK".to_string(),
            "/run/user/1000/ssh-agent.sock".to_string()
        )));
        assert!(!plan.unset_env.contains(&"SSH_AUTH_SOCK".to_string()));
    }

    #[test]
    fn test_build_mount_plan_without_ssh_socket() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("claude"),
            agent_args: vec![],
            no_ssh: false,
            with_toolchains: vec![],
        };

        let config = load_config();
        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, None, &[]);

        let target_sock = PathBuf::from("/run/user/1000/ssh-agent.sock");
        assert!(!plan.mounts.iter().any(|m| m.target_path == target_sock));
        assert!(!plan.set_env.iter().any(|(k, _)| k == "SSH_AUTH_SOCK"));
        assert!(plan.unset_env.contains(&"SSH_AUTH_SOCK".to_string()));
    }

    #[test]
    fn test_build_mount_plan_with_no_ssh_flag() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        let sock = dir.path().join("fake_agent.sock");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("claude"),
            agent_args: vec![],
            no_ssh: true,
            with_toolchains: vec![],
        };

        let config = load_config();
        // Caller (main.rs) passes None when no_ssh is true
        let ssh_socket = if opts.no_ssh { None } else { Some(sock.as_path()) };
        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, ssh_socket, &[]);

        // When no_ssh is true, socket is NOT mounted and SSH_AUTH_SOCK is unset
        let target_sock = PathBuf::from("/run/user/1000/ssh-agent.sock");
        assert!(!plan.mounts.iter().any(|m| m.target_path == target_sock));
        assert!(!plan.set_env.iter().any(|(k, _)| k == "SSH_AUTH_SOCK"));
        assert!(plan.unset_env.contains(&"SSH_AUTH_SOCK".to_string()));
    }

    #[test]
    fn test_tmpfs_skips_missing_host_paths() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("claude"),
            agent_args: vec![],
            no_ssh: false,
            with_toolchains: vec![],
        };

        let mut config = load_config();
        let missing_path = "/nonexistent/test_tmpfs_dir_abc123";
        config.defaults.tmpfs.push(missing_path.to_string());

        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, None, &[]);

        // Standard /tmp exists and is mounted
        assert!(plan.mounts.iter().any(|m| m.target_path == Path::new("/tmp") && m.kind == MountKind::Tmpfs));
        // Missing tmpfs path is skipped
        assert!(!plan.mounts.iter().any(|m| m.target_path == Path::new(missing_path)));
    }

    #[test]
    fn test_toolchain_config_parse() {
        let toml_str = r#"
            [defaults]
            ro_system = ["/usr"]
            ro_home = []
            rw_cache = []
            tmpfs = []

            [toolchains.mytool]
            requires = ["bin/mytool"]
            rw_cache = [".mytool/cache"]
            runtime = [".mytool/config"]
            check_tcp = [{ addr = "127.0.0.1:9999", hint = "run server" }]
        "#;
        let cfg: ConfigData = toml::from_str(toml_str).expect("toolchains parse cleanly");
        assert!(cfg.toolchains.contains_key("mytool"));
        let tc = &cfg.toolchains["mytool"];
        assert_eq!(tc.requires, vec!["bin/mytool"]);
        assert_eq!(tc.rw_cache, vec![".mytool/cache"]);
        assert_eq!(tc.runtime, vec![".mytool/config"]);
        assert_eq!(
            tc.check_tcp,
            vec![TcpCheckConfig {
                addr: "127.0.0.1:9999".into(),
                hint: "run server".into(),
            }]
        );
    }

    #[test]
    fn test_merge_user_config_toolchains() {
        let mut config = ConfigData {
            defaults: DefaultsConfig {
                ro_system: vec![],
                ro_home: vec![],
                rw_cache: vec![],
                tmpfs: vec![],
            },
            profiles: HashMap::new(),
            toolchains: HashMap::new(),
            sources: vec![],
        };
        config.toolchains.insert(
            "test_tc".into(),
            ToolchainConfig {
                requires: vec!["req1".into()],
                rw_cache: vec!["cache1".into(), "moved_to_ro".into()],
                runtime: vec!["ro1".into(), "moved_to_rw".into()],
                check_tcp: vec![],
            },
        );

        let mut user_config = UserConfigData::default();
        user_config.toolchains.insert(
            "test_tc".into(),
            UserToolchainConfig {
                requires: vec!["req2".into()],
                rw_cache: vec!["moved_to_rw".into(), "new_cache".into()],
                runtime: vec!["moved_to_ro".into(), "new_ro".into()],
                check_tcp: vec![TcpCheckConfig {
                    addr: "127.0.0.1:5037".into(),
                    hint: "start adb".into(),
                }],
            },
        );

        merge_user_config(&mut config, user_config);

        let merged = &config.toolchains["test_tc"];
        assert_eq!(merged.requires, vec!["req1", "req2"]);
        // moved_to_rw moved from runtime into rw_cache
        assert!(!merged.runtime.contains(&"moved_to_rw".to_string()));
        assert!(merged.rw_cache.contains(&"moved_to_rw".to_string()));
        assert!(merged.rw_cache.contains(&"new_cache".to_string()));

        // moved_to_ro moved from rw_cache into runtime
        assert!(!merged.rw_cache.contains(&"moved_to_ro".to_string()));
        assert!(merged.runtime.contains(&"moved_to_ro".to_string()));
        assert!(merged.runtime.contains(&"new_ro".to_string()));

        assert_eq!(merged.check_tcp.len(), 1);
        assert_eq!(merged.check_tcp[0].addr, "127.0.0.1:5037");
    }

    #[test]
    fn test_build_mount_plan_with_toolchains_literal() {
        let dir = tempdir().unwrap();
        let home = dir.path().join("home");
        let cwd = dir.path().join("project");
        let tc_cache = home.join(".mytool").join("cache");
        let tc_runtime = home.join(".mytool").join("sdk");
        let tc_uncreated = home.join(".mytool").join("missing_cache");
        fs::create_dir_all(&tc_cache).unwrap();
        fs::create_dir_all(&tc_runtime).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let opts = RunOptions {
            extra_dirs: vec![],
            agent_cmd: OsString::from("sh"),
            agent_args: vec![],
            no_ssh: false,
            with_toolchains: vec!["mytool".into()],
        };

        let mut config = load_config();
        config.toolchains.insert(
            "mytool".into(),
            ToolchainConfig {
                requires: vec![],
                rw_cache: vec![".mytool/cache".into(), ".mytool/missing_cache".into()],
                runtime: vec![".mytool/sdk".into()],
                check_tcp: vec![],
            },
        );

        let resolved_tc = resolve_literal_toolchain("mytool", &config.toolchains["mytool"], &home);
        let plan = build_mount_plan(&opts, &home, &cwd, 1000, &config, None, &[resolved_tc]);

        // Toolchain rw_cache mounted as BindReadWrite
        let cache_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == tc_cache && m.kind == MountKind::BindReadWrite)
            .expect("tc_cache mount must exist as BindReadWrite");

        // Toolchain runtime mounted as BindReadOnly
        let runtime_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == tc_runtime && m.kind == MountKind::BindReadOnly)
            .expect("tc_runtime mount must exist as BindReadOnly");

        // rw_cache mounted before runtime
        assert!(cache_idx < runtime_idx, "rw_cache must be mounted before runtime");

        // Uncreated cache path was NOT mounted and NOT created
        assert!(!plan.mounts.iter().any(|m| m.target_path == tc_uncreated));
        assert!(!tc_uncreated.exists(), "uncreated toolchain cache must never be created");

        // Toolchain mounted before working directory
        let canonical_cwd = fs::canonicalize(&cwd).unwrap();
        let cwd_idx = plan
            .mounts
            .iter()
            .position(|m| m.target_path == canonical_cwd)
            .expect("cwd mount must exist");
        assert!(runtime_idx < cwd_idx, "toolchains must be mounted before cwd");
    }

    #[test]
    fn test_example_profiles_toml_is_valid() {
        let example_str = include_str!("../examples/profiles.toml");
        let parsed: Result<UserConfigData, _> = toml::from_str(example_str);
        assert!(parsed.is_ok(), "examples/profiles.toml failed to parse: {:?}", parsed.err());
        let user_cfg = parsed.unwrap();
        assert!(user_cfg.profiles.contains_key("myagent"));
    }

    struct MockLookup {
        vars: HashMap<String, String>,
        bins: HashMap<String, PathBuf>,
    }

    impl ExpansionLookup for MockLookup {
        fn lookup_var(&self, var: &str) -> Option<String> {
            self.vars.get(var).cloned()
        }

        fn lookup_bin(&self, bin: &str) -> Option<PathBuf> {
            self.bins.get(bin).cloned()
        }
    }

    #[test]
    fn test_expansion_variable_set_and_unset() {
        let mut vars = HashMap::new();
        vars.insert("MY_DIR".to_string(), "custom_dir".to_string());
        let lookup = MockLookup {
            vars,
            bins: HashMap::new(),
        };

        // Variable set
        assert_eq!(
            expand_string("${MY_DIR}/sub", &lookup),
            Some("custom_dir/sub".to_string())
        );

        // Variable unset without default fails
        assert_eq!(expand_string("${UNSET_VAR}/sub", &lookup), None);
    }

    #[test]
    fn test_expansion_default_values() {
        let mut vars = HashMap::new();
        vars.insert("SET_VAR".to_string(), "present".to_string());
        vars.insert("EMPTY_VAR".to_string(), "".to_string());
        let lookup = MockLookup {
            vars,
            bins: HashMap::new(),
        };

        // When set, uses variable value
        assert_eq!(
            expand_string("${SET_VAR:-fallback}", &lookup),
            Some("present".to_string())
        );

        // When unset, uses default value
        assert_eq!(
            expand_string("${UNSET_VAR:-fallback}", &lookup),
            Some("fallback".to_string())
        );

        // When empty, uses default value
        assert_eq!(
            expand_string("${EMPTY_VAR:-fallback}", &lookup),
            Some("fallback".to_string())
        );
    }

    #[test]
    fn test_expansion_bin_lookup_and_missing() {
        let mut bins = HashMap::new();
        bins.insert(
            "flutter".to_string(),
            PathBuf::from("/opt/flutter/bin/flutter"),
        );
        let lookup = MockLookup {
            vars: HashMap::new(),
            bins,
        };

        // Found binary
        assert_eq!(
            expand_string("${bin:flutter}/../..", &lookup),
            Some("/opt/flutter/bin/flutter/../..".to_string())
        );

        // Missing binary fails
        assert_eq!(expand_string("${bin:nonexistent}", &lookup), None);
    }

    #[test]
    fn test_expand_entry_and_dot_dot_resolution() {
        let mut bins = HashMap::new();
        bins.insert(
            "flutter".to_string(),
            PathBuf::from("/opt/flutter/bin/flutter"),
        );
        let lookup = MockLookup {
            vars: HashMap::new(),
            bins,
        };
        let home = Path::new("/home/testuser");

        // ${bin:flutter}/../.. resolves lexically to /opt/flutter
        let p = expand_entry("${bin:flutter}/../..", home, &lookup).unwrap();
        assert_eq!(p, PathBuf::from("/opt/flutter"));

        // $HOME-relative path resolution
        let p2 = expand_entry(".pub-cache", home, &lookup).unwrap();
        assert_eq!(p2, PathBuf::from("/home/testuser/.pub-cache"));

        // Host-absolute path
        let p3 = expand_entry("/etc/custom", home, &lookup).unwrap();
        assert_eq!(p3, PathBuf::from("/etc/custom"));
    }

    #[test]
    fn test_symlinked_binary_resolution() {
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("real_bin");
        let symlink_dir = dir.path().join("symlink_dir");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&symlink_dir).unwrap();

        let real_binary = bin_dir.join("mybin");
        fs::write(&real_binary, "#!/bin/sh\n").unwrap();

        let symlink_path = symlink_dir.join("mybin");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_binary, &symlink_path).unwrap();

        // SystemLookup canonicalizes the binary path
        let canonical = fs::canonicalize(&symlink_path).unwrap();
        assert_eq!(canonical, fs::canonicalize(&real_binary).unwrap());

        let mut bins = HashMap::new();
        bins.insert("mybin".to_string(), canonical);
        let lookup = MockLookup {
            vars: HashMap::new(),
            bins,
        };

        let home = Path::new("/home/testuser");
        let expanded = expand_entry("${bin:mybin}/..", home, &lookup).unwrap();
        assert_eq!(expanded, fs::canonicalize(&bin_dir).unwrap());
    }

    #[test]
    fn test_check_tcp_port_listening_and_closed() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let addr = format!("127.0.0.1:{port}");

        // While listening, check_tcp_port returns true
        assert!(check_tcp_port(&addr, 50));

        // When closed, check_tcp_port returns false
        drop(listener);
        assert!(!check_tcp_port(&addr, 50));
    }
}
