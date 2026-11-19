use cave::bwrap::{check_bwrap_available, exec_bwrap, render_args};
use cave::cli::{parse_args, print_help, print_version, run_doctor, CliCommand};
use cave::policy::{
    agent_basename, build_mount_plan, check_tcp_port, load_effective_config, prepare_host_state,
    resolve_profile, resolve_ssh_socket, resolve_toolchain, select_profiles, validate_entry_status,
    MissingBehavior, SshSocket, SystemLookup,
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
        CliCommand::Doctor {
            with_toolchains,
            profiles,
        } => {
            if run_doctor(&with_toolchains, &profiles) {
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

            // 1. Check profile and toolchain names before resolving or mutating host state
            let selected_profiles = match select_profiles(&opts.agent_cmd, &opts.profiles, &config) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            };

            for name in &opts.with_toolchains {
                if !config.toolchains.contains_key(name) {
                    eprintln!("Error: unknown toolchain '{name}'");
                    return ExitCode::from(1);
                }
            }

            // 2. Resolve every entry of every selected profile and toolchain, and run all fatal checks
            let mut warnings = Vec::new();
            let mut resolved_profiles = Vec::new();
            for name in &selected_profiles {
                let report = resolve_profile(name, &config.profiles[name], &home, &SystemLookup);
                for s in &report.state {
                    if let Err(e) = validate_entry_status(
                        "profile",
                        name,
                        &s.raw,
                        &s.status,
                        MissingBehavior::Create,
                        &mut warnings,
                    ) {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                }
                for r in &report.runtime {
                    if let Err(e) = validate_entry_status(
                        "profile",
                        name,
                        &r.raw,
                        &r.status,
                        MissingBehavior::SkipSilently,
                        &mut warnings,
                    ) {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                }
                resolved_profiles.push(report.to_resolved_profile());
            }

            let mut resolved_toolchains = Vec::new();
            for name in &opts.with_toolchains {
                let tc = &config.toolchains[name];
                let report = resolve_toolchain(name, tc, &home, &SystemLookup);
                for req in &report.requires {
                    if let Err(e) = validate_entry_status(
                        "toolchain",
                        name,
                        &req.raw,
                        &req.status,
                        MissingBehavior::Fatal,
                        &mut warnings,
                    ) {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                }
                for entry in &report.rw_cache {
                    if let Err(e) = validate_entry_status(
                        "toolchain",
                        name,
                        &entry.raw,
                        &entry.status,
                        MissingBehavior::WarnAndSkip,
                        &mut warnings,
                    ) {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                }
                for entry in &report.runtime {
                    if let Err(e) = validate_entry_status(
                        "toolchain",
                        name,
                        &entry.raw,
                        &entry.status,
                        MissingBehavior::WarnAndSkip,
                        &mut warnings,
                    ) {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                }
                resolved_toolchains.push(report.to_resolved_toolchain());
            }

            // 3. Emit non-fatal warnings and perform TCP checks now that all fatal checks passed
            let agent_name = agent_basename(&opts.agent_cmd);
            if opts.profiles.is_empty() && !config.profiles.contains_key(agent_name) {
                eprintln!(
                    "Warning: no profile for agent '{agent_name}'; $HOME is ephemeral and state will not persist. Use --dir to mount host paths."
                );
            }

            for w in &warnings {
                eprintln!("{w}");
            }

            for name in &opts.with_toolchains {
                for tcp in &config.toolchains[name].check_tcp {
                    if !check_tcp_port(&tcp.addr, 50) {
                        eprintln!(
                            "Warning: toolchain '{name}': nothing listening on {}; {}",
                            tcp.addr, tcp.hint
                        );
                    }
                }
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

            // 4. Explicit host side-effect: prepare declared state directories/files
            if let Err(e) = prepare_host_state(&resolved_profiles) {
                eprintln!("Error: failed to prepare host state paths: {e}");
                return ExitCode::from(1);
            }

            // 5. Pure policy evaluation: build mount and environment plan
            let plan = build_mount_plan(
                &opts,
                &home,
                &cwd,
                uid,
                &config,
                ssh_socket.as_deref(),
                &resolved_profiles,
                &resolved_toolchains,
            );
            let bwrap_args = render_args(&plan, &opts.agent_cmd, &opts.agent_args);

            let err = exec_bwrap(&bwrap_bin, &bwrap_args);
            eprintln!("Error executing bwrap: {err}");
            ExitCode::from(1)
        }
    }
}
