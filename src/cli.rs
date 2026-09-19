use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CliCommand {
    Run(RunOptions),
    Help,
    Version,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RunOptions {
    pub dirs: Vec<PathBuf>,
    pub ro_dirs: Vec<PathBuf>,
    pub agent_cmd: String,
    pub agent_args: Vec<String>,
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
                write!(f, "Error: missing agent command. Usage: cave [options] <agent> [args...]")
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

pub fn parse_args<I, S>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut iter = args.into_iter();
    let mut dirs = Vec::new();
    let mut ro_dirs = Vec::new();

    while let Some(arg) = iter.next() {
        let s = arg.as_ref();
        match s {
            "--help" | "-h" => return Ok(CliCommand::Help),
            "--version" | "-V" => return Ok(CliCommand::Version),
            "--dir" => {
                let val = iter
                    .next()
                    .ok_or_else(|| CliError::MissingOptionValue("--dir".into()))?;
                let path = PathBuf::from(val.as_ref());
                validate_path(&path)?;
                check_warning(&path);
                dirs.push(path);
            }
            "--ro-dir" => {
                let val = iter
                    .next()
                    .ok_or_else(|| CliError::MissingOptionValue("--ro-dir".into()))?;
                let path = PathBuf::from(val.as_ref());
                validate_path(&path)?;
                check_warning(&path);
                ro_dirs.push(path);
            }
            "--" => {
                let agent_cmd = iter
                    .next()
                    .ok_or(CliError::MissingAgentCommand)?
                    .as_ref()
                    .to_string();
                let agent_args = iter.map(|a| a.as_ref().to_string()).collect();
                return Ok(CliCommand::Run(RunOptions {
                    dirs,
                    ro_dirs,
                    agent_cmd,
                    agent_args,
                }));
            }
            opt if opt.starts_with('-') => {
                return Err(CliError::UnknownOption(opt.to_string()));
            }
            agent_cmd => {
                let agent_args = iter.map(|a| a.as_ref().to_string()).collect();
                return Ok(CliCommand::Run(RunOptions {
                    dirs,
                    ro_dirs,
                    agent_cmd: agent_cmd.to_string(),
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
    if path == Path::new("/") {
        eprintln!("Warning: --dir / exposes the host root filesystem, defeating sandbox boundaries.");
    } else if path == Path::new("~") {
        eprintln!("Warning: --dir ~ exposes the entire host home directory, defeating sandbox boundaries.");
    } else if let Ok(home) = std::env::var("HOME") {
        if path == Path::new(&home) {
            eprintln!("Warning: --dir {} exposes the entire host home directory, defeating sandbox boundaries.", path.display());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_and_version() {
        assert_eq!(parse_args(["--help"]).unwrap(), CliCommand::Help);
        assert_eq!(parse_args(["-h"]).unwrap(), CliCommand::Help);
        assert_eq!(parse_args(["--version"]).unwrap(), CliCommand::Version);
        assert_eq!(parse_args(["-V"]).unwrap(), CliCommand::Version);
    }

    #[test]
    fn test_stop_at_first_non_option() {
        let args = ["--dir", "/tmp", "claude", "--dir", "another_thing"];
        let cmd = parse_args(args).unwrap();
        match cmd {
            CliCommand::Run(opts) => {
                assert_eq!(opts.dirs, vec![PathBuf::from("/tmp")]);
                assert_eq!(opts.agent_cmd, "claude");
                assert_eq!(opts.agent_args, vec!["--dir", "another_thing"]);
            }
            _ => panic!("Expected CliCommand::Run"),
        }
    }

    #[test]
    fn test_repeatable_dirs() {
        let args = ["--dir", "/tmp", "--ro-dir", "/usr", "sh", "-c", "echo hi"];
        let cmd = parse_args(args).unwrap();
        match cmd {
            CliCommand::Run(opts) => {
                assert_eq!(opts.dirs, vec![PathBuf::from("/tmp")]);
                assert_eq!(opts.ro_dirs, vec![PathBuf::from("/usr")]);
                assert_eq!(opts.agent_cmd, "sh");
                assert_eq!(opts.agent_args, vec!["-c", "echo hi"]);
            }
            _ => panic!("Expected CliCommand::Run"),
        }
    }

    #[test]
    fn test_missing_agent_command() {
        assert_eq!(
            parse_args::<_, &str>([]).unwrap_err(),
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
