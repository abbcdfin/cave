use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExtraDir {
    ReadWrite(PathBuf),
    ReadOnly(PathBuf),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CliCommand {
    Run(RunOptions),
    Help,
    Version,
    Doctor,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RunOptions {
    pub extra_dirs: Vec<ExtraDir>,
    pub agent_cmd: OsString,
    pub agent_args: Vec<OsString>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    MissingAgentCommand,
    UnknownOption(String),
    MissingOptionValue(String),
    PathNotFound(PathBuf),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::MissingAgentCommand => {
                write!(
                    f,
                    "Error: missing agent command. Usage: cave [options] <agent> [args...]"
                )
            }
            CliError::UnknownOption(opt) => write!(f, "Error: unknown option '{opt}'"),
            CliError::MissingOptionValue(opt) => {
                write!(f, "Error: option '{opt}' requires a path argument")
            }
            CliError::PathNotFound(path) => {
                write!(f, "Error: path does not exist: {}", path.display())
            }
        }
    }
}

impl std::error::Error for CliError {}

pub fn parse_args<I, T>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut extra_dirs = Vec::new();

    while let Some(arg) = iter.next() {
        let s = arg.to_str();
        match s {
            Some("--help") | Some("-h") => return Ok(CliCommand::Help),
            Some("--version") | Some("-V") => return Ok(CliCommand::Version),
            Some("--doctor") => return Ok(CliCommand::Doctor),
            Some("--dir") => {
                let val = iter
                    .next()
                    .ok_or_else(|| CliError::MissingOptionValue("--dir".into()))?;
                let path = PathBuf::from(val);
                validate_path(&path)?;
                check_warning(&path);
                extra_dirs.push(ExtraDir::ReadWrite(path));
            }
            Some("--ro-dir") => {
                let val = iter
                    .next()
                    .ok_or_else(|| CliError::MissingOptionValue("--ro-dir".into()))?;
                let path = PathBuf::from(val);
                validate_path(&path)?;
                check_warning(&path);
                extra_dirs.push(ExtraDir::ReadOnly(path));
            }
            Some("--") => {
                let agent_cmd = iter
                    .next()
                    .ok_or(CliError::MissingAgentCommand)?;
                let agent_args = iter.collect();
                return Ok(CliCommand::Run(RunOptions {
                    extra_dirs,
                    agent_cmd,
                    agent_args,
                }));
            }
            Some(opt) if opt.starts_with('-') => {
                return Err(CliError::UnknownOption(opt.to_string()));
            }
            _ => {
                // First non-option (or non-UTF8 option-like) is the agent command
                let agent_cmd = arg;
                let agent_args = iter.collect();
                return Ok(CliCommand::Run(RunOptions {
                    extra_dirs,
                    agent_cmd,
                    agent_args,
                }));
            }
        }
    }

    Err(CliError::MissingAgentCommand)
}

fn validate_path(path: &Path) -> Result<(), CliError> {
    if !path.exists() {
        Err(CliError::PathNotFound(path.to_path_buf()))
    } else {
        Ok(())
    }
}

fn check_warning(path: &Path) {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if canonical == Path::new("/") {
        eprintln!("Warning: --dir / exposes the host root filesystem, defeating sandbox boundaries.");
    } else if let Ok(home) = std::env::var("HOME") {
        let canonical_home = fs::canonicalize(&home).unwrap_or_else(|_| PathBuf::from(home));
        if canonical == canonical_home {
            eprintln!(
                "Warning: --dir {} exposes the entire host home directory, defeating sandbox boundaries.",
                path.display()
            );
        }
    }
}

pub fn print_help() {
    println!(
        r#"cave — Transparent sandbox runner for coding agents in YOLO mode

Usage:
    cave [cave-options] <agent> [args...]

Options:
    --dir PATH       Mount PATH read-write at its host path (repeatable)
    --ro-dir PATH    Mount PATH read-only at its host path (repeatable)
    --doctor         Check system prerequisites and sandbox capabilities
    -h, --help       Print help information
    -V, --version    Print version information

Examples:
    cave claude --dangerously-skip-permissions
    cave claude -c
    cave --dir /tmp claude
    cave --ro-dir /opt/libs aider
"#
    );
}

pub fn print_version() {
    println!("cave {}", env!("CARGO_PKG_VERSION"));
}

pub fn run_doctor() -> bool {
    println!("cave doctor: inspecting system prerequisites and sandbox environment...\n");
    let mut ok = true;

    // 1. Check bwrap binary
    match crate::bwrap::check_bwrap_available() {
        Ok(path) => println!("  [OK] bubblewrap found at {}", path.display()),
        Err(e) => {
            println!("  [FAIL] bubblewrap: {e}");
            ok = false;
        }
    }

    // 2. Check unprivileged user namespaces sysctls
    let userns_proc = Path::new("/proc/sys/kernel/unprivileged_userns_clone");
    if userns_proc.exists() {
        if let Ok(val) = fs::read_to_string(userns_proc) {
            if val.trim() == "1" {
                println!("  [OK] unprivileged user namespaces enabled (/proc/sys/kernel/unprivileged_userns_clone)");
            } else {
                println!("  [WARN] /proc/sys/kernel/unprivileged_userns_clone is {}", val.trim());
            }
        }
    } else {
        println!("  [OK] unprivileged user namespaces available (unprivileged_userns_clone procfs absent)");
    }

    // Ubuntu 24.04 AppArmor restriction
    let apparmor_userns = Path::new("/proc/sys/kernel/apparmor_restrict_unprivileged_userns");
    if let Ok(val) = fs::read_to_string(apparmor_userns) {
        if val.trim() == "1" {
            println!("  [WARN] /proc/sys/kernel/apparmor_restrict_unprivileged_userns is 1 (AppArmor may restrict user namespaces)");
        } else {
            println!("  [OK] AppArmor unprivileged user namespace restriction disabled");
        }
    }

    // 3. Check HOME
    let home_path = if let Ok(home) = std::env::var("HOME") {
        println!("  [OK] HOME directory: {home}");
        Some(PathBuf::from(home))
    } else {
        println!("  [FAIL] HOME environment variable is not set");
        ok = false;
        None
    };

    // 4. Check /var/run is a symlink to /run (Docker socket leak check)
    let var_run = Path::new("/var/run");
    if let Ok(meta) = fs::symlink_metadata(var_run) {
        if meta.file_type().is_symlink() {
            println!("  [OK] /var/run is a symlink to /run (Docker socket not exposed via /var bind)");
        } else {
            println!("  [WARN] /var/run is a real directory, not a symlink! Read-only /var bind may expose host Docker socket.");
        }
    } else {
        println!("  [OK] /var/run is absent or inaccessible");
    }

    // 5. Test DNS resolution inside sandbox
    if let Some(ref home) = home_path {
        let bwrap_res = crate::bwrap::check_bwrap_available();
        if let Ok(bwrap_bin) = bwrap_res {
            let test_opts = RunOptions {
                extra_dirs: vec![],
                agent_cmd: std::ffi::OsString::from("sh"),
                agent_args: vec![
                    std::ffi::OsString::from("-c"),
                    std::ffi::OsString::from("getent ahostsv4 localhost || getent hosts localhost || true"),
                ],
            };
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
            let uid = unsafe { libc::getuid() };
            let plan = crate::policy::build_mount_plan(&test_opts, home, &cwd, uid);
            let bwrap_args = crate::bwrap::render_args(&plan, &test_opts.agent_cmd, &test_opts.agent_args);
            let check_output = std::process::Command::new(bwrap_bin)
                .args(&bwrap_args)
                .output();
            match check_output {
                Ok(out) if out.status.success() && String::from_utf8_lossy(&out.stdout).contains("127.0.0.1") => {
                    println!("  [OK] sandbox DNS resolution verified (resolved localhost to 127.0.0.1)");
                }
                Ok(out) => {
                    println!("  [WARN] sandbox DNS resolution returned non-zero or unexpected output: {:?}", out.status.code());
                }
                Err(e) => {
                    println!("  [WARN] sandbox DNS resolution check failed to run: {e}");
                }
            }
        }
    }

    // 6. Print loaded configuration sources and effective merged table
    println!();
    if let Some(ref home) = home_path {
        match crate::policy::load_effective_config(home) {
            Ok(config) => {
                println!("Loaded configuration sources:");
                for src in &config.sources {
                    println!("  - {src}");
                }
                println!();
                println!("Effective merged configuration:");
                println!("  Defaults:");
                println!("    ro_system ({} paths): {:?}", config.defaults.ro_system.len(), config.defaults.ro_system);
                println!("    ro_home   ({} paths): {:?}", config.defaults.ro_home.len(), config.defaults.ro_home);
                println!("    rw_cache  ({} paths): {:?}", config.defaults.rw_cache.len(), config.defaults.rw_cache);
                println!("    tmpfs     ({} paths): {:?}", config.defaults.tmpfs.len(), config.defaults.tmpfs);
                println!("  Profiles:");
                let mut profile_names: Vec<_> = config.profiles.keys().collect();
                profile_names.sort();
                for name in profile_names {
                    let prof = &config.profiles[name];
                    println!("    [{name}]");
                    if !prof.runtime.is_empty() {
                        println!("      runtime (RO): {:?}", prof.runtime);
                    }
                    if !prof.state.is_empty() {
                        let state_repr: Vec<_> = prof.state.iter().map(|s| format!("{} ({})", s.path, s.kind)).collect();
                        println!("      state   (RW): {:?}", state_repr);
                    }
                }
            }
            Err(e) => {
                println!("  [FAIL] Failed to load configuration: {e}");
                ok = false;
            }
        }
    }

    println!();
    if ok {
        println!("System is ready to run cave.");
    } else {
        println!("Some prerequisites are missing. Please address the errors above.");
    }

    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_version_doctor() {
        assert_eq!(parse_args(["--help"]).unwrap(), CliCommand::Help);
        assert_eq!(parse_args(["-h"]).unwrap(), CliCommand::Help);
        assert_eq!(parse_args(["--version"]).unwrap(), CliCommand::Version);
        assert_eq!(parse_args(["-V"]).unwrap(), CliCommand::Version);
        assert_eq!(parse_args(["--doctor"]).unwrap(), CliCommand::Doctor);
    }

    #[test]
    fn test_preserves_command_line_order() {
        let args = ["--ro-dir", "/usr", "--dir", "/tmp", "claude", "--flag"];
        let cmd = parse_args(args).unwrap();
        match cmd {
            CliCommand::Run(opts) => {
                assert_eq!(
                    opts.extra_dirs,
                    vec![
                        ExtraDir::ReadOnly(PathBuf::from("/usr")),
                        ExtraDir::ReadWrite(PathBuf::from("/tmp")),
                    ]
                );
                assert_eq!(opts.agent_cmd, OsString::from("claude"));
                assert_eq!(opts.agent_args, vec![OsString::from("--flag")]);
            }
            _ => panic!("Expected CliCommand::Run"),
        }
    }

    #[test]
    fn test_preserves_raw_os_string_bytes() {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let non_utf8_arg = OsString::from_vec(vec![b'x', 0xff]);
            let args = vec![OsString::from("claude"), non_utf8_arg.clone()];
            let cmd = parse_args(args).unwrap();
            match cmd {
                CliCommand::Run(opts) => {
                    assert_eq!(opts.agent_args[0], non_utf8_arg);
                }
                _ => panic!("Expected CliCommand::Run"),
            }
        }
    }

    #[test]
    fn test_missing_agent_command() {
        assert_eq!(
            parse_args::<_, OsString>([]).unwrap_err(),
            CliError::MissingAgentCommand
        );
        assert_eq!(
            parse_args(["--dir", "/tmp"]).unwrap_err(),
            CliError::MissingAgentCommand
        );
    }

    #[test]
    fn test_unknown_option() {
        assert_eq!(
            parse_args(["--bogus", "claude"]).unwrap_err(),
            CliError::UnknownOption("--bogus".into())
        );
    }

    #[test]
    fn test_missing_option_value() {
        assert_eq!(
            parse_args(["--dir"]).unwrap_err(),
            CliError::MissingOptionValue("--dir".into())
        );
        assert_eq!(
            parse_args(["--ro-dir"]).unwrap_err(),
            CliError::MissingOptionValue("--ro-dir".into())
        );
    }

    #[test]
    fn test_nonexistent_path() {
        let bad_path = "/nonexistent_path_xyz_123456";
        assert_eq!(
            parse_args(["--dir", bad_path, "claude"]).unwrap_err(),
            CliError::PathNotFound(PathBuf::from(bad_path))
        );
    }
}
