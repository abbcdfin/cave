use crate::policy::{MountKind, MountPlan};
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum BwrapError {
    NotFound(String),
    ExecFailed(std::io::Error),
}

impl std::fmt::Display for BwrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BwrapError::NotFound(msg) => write!(f, "{msg}"),
            BwrapError::ExecFailed(err) => write!(f, "Failed to exec bwrap: {err}"),
        }
    }
}

impl std::error::Error for BwrapError {}

pub fn check_bwrap_available() -> Result<PathBuf, BwrapError> {
    if let Ok(path) = which_bwrap() {
        Ok(path)
    } else {
        Err(BwrapError::NotFound(
            "bwrap is not installed or not found on PATH. Please install bubblewrap (e.g. 'sudo apt install bubblewrap' or 'sudo pacman -S bubblewrap').".to_string(),
        ))
    }
}

fn which_bwrap() -> Result<PathBuf, ()> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = PathBuf::from(dir).join("bwrap");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    if Path::new("/usr/bin/bwrap").is_file() {
        return Ok(PathBuf::from("/usr/bin/bwrap"));
    }
    Err(())
}

pub fn render_args(plan: &MountPlan, agent_cmd: &str, agent_args: &[String]) -> Vec<OsString> {
    let mut args = Vec::new();

    // Lifeline & standard namespaces
    args.push(OsString::from("--die-with-parent"));
    args.push(OsString::from("--unshare-pid"));
    args.push(OsString::from("--unshare-uts"));
    args.push(OsString::from("--unshare-ipc"));

    // Render mounts in order
    for mount in &plan.mounts {
        match mount.kind {
            MountKind::Proc => {
                args.push(OsString::from("--proc"));
                args.push(OsString::from("/proc"));
            }
            MountKind::Dev => {
                args.push(OsString::from("--dev"));
                args.push(OsString::from("/dev"));
            }
            MountKind::Tmpfs => {
                args.push(OsString::from("--tmpfs"));
                args.push(mount.target_path.as_os_str().to_os_string());
            }
            MountKind::BindReadOnly => {
                if let Some(ref host_p) = mount.host_path {
                    args.push(OsString::from("--ro-bind"));
                    args.push(host_p.as_os_str().to_os_string());
                    args.push(mount.target_path.as_os_str().to_os_string());
                }
            }
            MountKind::BindReadWrite => {
                if let Some(ref host_p) = mount.host_path {
                    args.push(OsString::from("--bind"));
                    args.push(host_p.as_os_str().to_os_string());
                    args.push(mount.target_path.as_os_str().to_os_string());
                }
            }
        }
    }

    // Isolated runtime dir for user
    let user_run = format!("/run/user/{}", plan.uid);
    args.push(OsString::from("--dir"));
    args.push(OsString::from(&user_run));
    args.push(OsString::from("--chmod"));
    args.push(OsString::from("0700"));
    args.push(OsString::from(&user_run));

    // Working directory
    args.push(OsString::from("--chdir"));
    args.push(plan.workdir.as_os_str().to_os_string());

    // Environment unsets
    for var in &plan.unset_env {
        args.push(OsString::from("--unsetenv"));
        args.push(OsString::from(var));
    }

    // Environment sets
    for (key, val) in &plan.set_env {
        args.push(OsString::from("--setenv"));
        args.push(OsString::from(key));
        args.push(OsString::from(val));
    }

    // Command separator and agent invocation
    args.push(OsString::from("--"));
    args.push(OsString::from(agent_cmd));
    for a in agent_args {
        args.push(OsString::from(a));
    }

    args
}

pub fn exec_bwrap(bwrap_bin: &Path, args: &[OsString]) -> std::io::Error {
    let mut cmd = Command::new(bwrap_bin);
    cmd.args(args);
    cmd.exec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_bwrap_available() {
        assert!(check_bwrap_available().is_ok());
    }
}
