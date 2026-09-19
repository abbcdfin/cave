use cave::bwrap::{check_bwrap_available, exec_bwrap, render_args};
use cave::cli::{parse_args, print_help, print_version, CliCommand};
use cave::policy::build_mount_plan;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = match parse_args(&args) {
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
        CliCommand::Run(opts) => {
            let bwrap_bin = match check_bwrap_available() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return ExitCode::from(1);
                }
            };

            let home = env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/"));
            let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let uid = unsafe { libc::getuid() };

            let plan = build_mount_plan(&opts, &home, &cwd, uid);
            let bwrap_args = render_args(&plan, &opts.agent_cmd, &opts.agent_args);

            let err = exec_bwrap(&bwrap_bin, &bwrap_args);
            eprintln!("Error executing bwrap: {err}");
            ExitCode::from(1)
        }
    }
}
