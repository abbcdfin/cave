use cave::bwrap::{check_bwrap_available, exec_bwrap, render_args};
use cave::cli::{parse_args, print_help, print_version, run_doctor, CliCommand};
use cave::policy::{
    agent_basename, build_mount_plan, load_effective_config, prepare_host_state,
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
        CliCommand::Doctor => {
            if run_doctor() {
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

            // Warn if no profile matches agent basename (unknown agent fail-loudly policy)
            let agent_name = agent_basename(&opts.agent_cmd);
            if !config.profiles.contains_key(agent_name) {
                eprintln!(
                    "Warning: no profile for agent '{agent_name}'; $HOME is ephemeral and state will not persist. Use --dir to mount host paths."
                );
            }

            // Explicit host side-effect: prepare declared state directories/files
            if let Err(e) = prepare_host_state(&opts.agent_cmd, &home, &config) {
                eprintln!("Error: failed to prepare host state paths: {e}");
                return ExitCode::from(1);
            }

            // Pure policy evaluation: build mount and environment plan
            let plan = build_mount_plan(&opts, &home, &cwd, uid, &config);
            let bwrap_args = render_args(&plan, &opts.agent_cmd, &opts.agent_args);

            let err = exec_bwrap(&bwrap_bin, &bwrap_args);
            eprintln!("Error executing bwrap: {err}");
            ExitCode::from(1)
        }
    }
}
