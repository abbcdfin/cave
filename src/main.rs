use cave::bwrap::{check_bwrap_available, exec_bwrap, render_args};
use cave::cli::{parse_args, print_help, print_version, run_doctor, CliCommand};
use cave::policy::{
    agent_basename, build_mount_plan, load_effective_config, prepare_host_state,
    resolve_ssh_socket, SshSocket,
};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args_os().skip(1);

    let command = match parse_args(args) {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    match command {
        CliCommand::Help => {
            print_help();
            ExitCode::SUCCESS
        }
        CliCommand::Version => {
            print_version();
            ExitCode::SUCCESS
        }
        CliCommand::Doctor { with_toolchains } => {
            if run_doctor(&with_toolchains) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        CliCommand::Run(opts) => {
            let bwrap_bin = match check_bwrap_available() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return ExitCode::from(1);
                }
            };

            let home_os = match env::var_os("HOME") {
                Some(h) if !h.is_empty() => h,
                _ => {
                    eprintln!("Error: HOME environment variable is not set");
                    return ExitCode::from(1);
                }
            };
            let home = PathBuf::from(home_os);

            let cwd = match env::current_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("Error: failed to determine current working directory: {e}");
                    return ExitCode::from(1);
                }
            };
            let uid = unsafe { libc::getuid() };

            let config = match load_effective_config(&home) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            };

            // Unknown toolchain check
            for name in &opts.with_toolchains {
                if !config.toolchains.contains_key(name) {
                    eprintln!("Error: unknown toolchain '{name}'");
                    return ExitCode::from(1);
                }
            }

            // Warn if no profile matches agent basename (unknown agent fail-loudly policy)
            let agent_name = agent_basename(&opts.agent_cmd);
            if !config.profiles.contains_key(agent_name) {
                eprintln!(
                    "Warning: no profile for agent '{agent_name}'; $HOME is ephemeral and state will not persist. Use --dir to mount host paths."
                );
            }

            // Resolve host SSH agent socket if available and not disabled via --no-ssh
            let ssh_socket = if opts.no_ssh {
                None
            } else {
                match resolve_ssh_socket() {
                    SshSocket::Valid(p) => Some(p),
                    SshSocket::Stale(_) => {
                        eprintln!(
                            "Warning: SSH_AUTH_SOCK points to a missing socket; ssh-agent not forwarded."
                        );
                        None
                    }
                    SshSocket::Unset => None,
                }
            };

            // Explicit host side-effect: prepare declared state directories/files
            if let Err(e) = prepare_host_state(&opts.agent_cmd, &home, &config) {
                eprintln!("Error: failed to prepare host state paths: {e}");
                return ExitCode::from(1);
            }

            // Resolve toolchains
            let mut resolved_toolchains = Vec::new();
            for name in &opts.with_toolchains {
                let tc = &config.toolchains[name];
                let report = cave::policy::resolve_toolchain(name, tc, &home, &cave::policy::SystemLookup);

                // Fatal check: any entry resolving to /, $HOME, or containing $HOME is fatal
                let all_entries = report
                    .requires
                    .iter()
                    .chain(report.rw_cache.iter())
                    .chain(report.runtime.iter());
                for entry in all_entries {
                    if let cave::policy::EntryResolution::InvalidRootOrHome(ref p) = entry.status {
                        eprintln!(
                            "Error: toolchain '{name}' entry '{}' resolves to '{}', which is invalid (cannot mount /, $HOME, or a folder containing $HOME).",
                            entry.raw,
                            p.display()
                        );
                        return ExitCode::from(1);
                    }
                }

                // Fatal check: any requires entry that does not resolve or does not exist is fatal
                for req in &report.requires {
                    match &req.status {
                        cave::policy::EntryResolution::Found(_) => {}
                        _ => {
                            eprintln!("Error: toolchain '{name}' requires '{}', which could not be found.", req.raw);
                            return ExitCode::from(1);
                        }
                    }
                }

                // Check TCP services: warn at startup if nothing listens on addr
                for tcp in &tc.check_tcp {
                    if !cave::policy::check_tcp_port(&tcp.addr, 50) {
                        eprintln!(
                            "Warning: toolchain '{name}': nothing listening on {}; {}",
                            tcp.addr, tcp.hint
                        );
                    }
                }

                // Warn on unresolvable/missing rw_cache and runtime entries
                for entry in &report.rw_cache {
                    if !matches!(entry.status, cave::policy::EntryResolution::Found(_)) {
                        eprintln!("Warning: toolchain '{name}' entry '{}' does not resolve; skipping.", entry.raw);
                    }
                }
                for entry in &report.runtime {
                    if !matches!(entry.status, cave::policy::EntryResolution::Found(_)) {
                        eprintln!("Warning: toolchain '{name}' entry '{}' does not resolve; skipping.", entry.raw);
                    }
                }

                resolved_toolchains.push(report.to_resolved_toolchain());
            }

            // Pure policy evaluation: build mount and environment plan
            let plan = build_mount_plan(
                &opts,
                &home,
                &cwd,
                uid,
                &config,
                ssh_socket.as_deref(),
                &resolved_toolchains,
            );
            let bwrap_args = render_args(&plan, &opts.agent_cmd, &opts.agent_args);

            let err = exec_bwrap(&bwrap_bin, &bwrap_args);
            eprintln!("Error executing bwrap: {err}");
            ExitCode::from(1)
        }
    }
}
