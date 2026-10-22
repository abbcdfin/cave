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
    if let Ok(home) = std::env::var("HOME") {
        println!("  [OK] HOME directory: {home}");
    } else {
        println!("  [FAIL] HOME environment variable is not set");
        ok = false;
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
