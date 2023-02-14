use std::{
    ffi::OsString, io::BufRead, net::TcpListener, path::PathBuf, thread::JoinHandle, time::Duration,
};

use duct::cmd;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
enum Command {
    FmtLint,
    BuildAll,
    RunServerAndClient,
    RunServer,
}

impl Command {
    fn dispatch(self) -> Result<(), std::io::Error> {
        println!("xtask : {:?}", self);
        match self {
            Command::FmtLint => fmt_and_lint(),
            Command::BuildAll => {
                fmt_and_lint()?;
                cargo_build_all()
            }
            Command::RunServer => cargo_run_server(),
            Command::RunServerAndClient => cargo_run_server_and_client(),
        }
    }
}

#[derive(StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    cmd: Option<Command>,
}

fn main() {
    let opts = Opts::from_args();
    std::env::set_var("RUST_BACKTRACE", "full");

    if port_already_bound(8081) {
        println!("daemon port already bound");
        return;
    }

    set_required_rustflags(&["-Wunused-crate-dependencies"]);
    enforce_root_dir_is_cwd();
    opts.cmd
        .unwrap_or_else(|| panic!("no command given"))
        .dispatch()
        .unwrap_or_else(|err| panic!("error with xtask, {err:?}"));
}

fn port_already_bound(port: u16) -> bool {
    TcpListener::bind("127.0.0.1:8080").is_err()
}

// Panic if we aren't in the project root.
fn enforce_root_dir_is_cwd() {
    let current_dir = std::env::current_dir().unwrap();
    let mut project_root = env!("CARGO_MANIFEST_DIR").parse::<PathBuf>().unwrap();
    project_root.pop();
    assert_eq!(
        current_dir, project_root,
        "xtask must be called from project root: {project_root:?}"
    );
}

// Append passed flags onto rustflags for compilation.
fn set_required_rustflags(flags: &[&'static str]) {
    let existing_rustflags = std::env::var("RUSTFLAGS").unwrap_or_else(|_| "".to_owned());
    let additional = flags.iter().map(|s| format!("{s} ")).collect::<String>();
    std::env::set_var("RUSTFLAGS", format!("{existing_rustflags} {additional}"));
}

// TODO: format dry run to break as an error
fn fmt_and_lint() -> Result<(), std::io::Error> {
    let output = cmd!("cargo", "+nightly", "fmt").run()?;
    println!("xtask fmt {}", String::from_utf8_lossy(&output.stdout));
    let output = cmd!("cargo", "clippy").run()?;
    println!("xtask clippy {}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

fn cargo_build_all() -> Result<(), std::io::Error> {
    let output = cmd!("cargo", "build").run()?;
    println!(
        "xtask build-all {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

fn cargo_x_bin(cargo_cmd: &str, bin_name: &str) -> Result<(), std::io::Error> {
    let output = cmd!("cargo", cargo_cmd, "--bin", bin_name).run()?;
    println!("xtask fmt {}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}

fn cargo_run_server_and_client() -> Result<(), std::io::Error> {
    println!("started server");
    let daemon = cargo_run_and_read("daemon")?;

    std::thread::sleep(Duration::from_millis(1000));

    println!("started client");
    let client = cargo_run_and_read("client")?;

    client.join().expect("unable to join thread")?;
    daemon.join().expect("unable to join thread")?;
    Ok(())
}

fn cargo_run_server() -> Result<(), std::io::Error> {
    let daemon = cargo_run_and_read("daemon")?;
    daemon.join().expect("unable to join thread")?;
    Ok(())
}

fn cargo_run_and_read(
    bin_name: &str,
) -> Result<JoinHandle<Result<(), std::io::Error>>, std::io::Error> {
    let args = vec!["run", "--bin", bin_name, "--"];
    let args = args.iter().map(Into::<OsString>::into).collect::<Vec<_>>();
    let proc = duct::cmd("cargo", args).reader()?;
    Ok(std::thread::spawn(move || {
        for line in std::io::BufReader::new(&proc).lines() {
            let pids = proc.pids();
            println!("{:?}: {}", pids, line?);
        }
        Ok(())
    }))
}
