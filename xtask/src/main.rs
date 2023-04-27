use std::{ffi::OsString, io::BufRead, net::TcpListener, path::PathBuf, thread::JoinHandle};

use duct::cmd;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
enum Command {
    /// Format and lint the code in this project.
    FmtLint,
    /// Build all binaries in this project (client and server).
    BuildAll,
    /// Run just the daemon, useful for testing.
    RunDaemon,
    /// Generate a self-signed certificate and key for the agent.
    GenerateSelfSignedCert { hostname: String },
    /// Create a dist tarball of the agent, with a given version number provided (manual).
    Dist {
        version: u32,
        #[structopt(short, long)]
        regenerate_key_and_certificate: bool,
    },
    /// Clean the dist directory.
    CleanDist,
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
            Command::RunDaemon => cargo_run_server(),
            Command::GenerateSelfSignedCert { hostname } => generate_cert_and_key_files(&hostname),
            Command::CleanDist => {
                cmd!("rm", "-rf", "target/dist").run()?;
                Ok(())
            }
            Command::Dist {
                version,
                regenerate_key_and_certificate,
            } => create_dist_tarball(version, regenerate_key_and_certificate),
        }
    }
}

/// Create a tarball of the agent, with a given version number provided (manual)
fn create_dist_tarball(
    version: u32,
    regenerate_key_and_certificate: bool,
) -> Result<(), std::io::Error> {
    if regenerate_key_and_certificate {
        cmd!(
            "cargo",
            "xtask",
            "generate-self-signed-cert",
            format!("agent")
        )
        .run()?;
    }

    cmd!("cargo", "build", "--release").run()?;
    cmd!("mkdir", "-p", "target/dist/assets").run()?;
    // copy artifacts to dist dir.
    cmd!("cp", "target/release/daemon", "target/dist").run()?;
    cmd!("cp", "target/release/client", "target/dist").run()?;
    cmd!("cp", format!("assets/agent-crt.pem"), "target/dist/assets/").run()?;
    cmd!("cp", format!("assets/agent-key.pem"), "target/dist/assets/").run()?;

    // tar up dist file. Will be compressed with zstd when sent.
    println!("Creating tar file:");
    cmd!(
        "tar",
        "-cvf",
        format!("target/dist-{version}.tar"),
        "-C",
        "target",
        "dist",
    )
    .run()?;
    println!("agent dist tarball created in target/dist-{version}.tar");
    Ok(())
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
        println!("warning daemon port is already bound");
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
    TcpListener::bind(format!("0.0.0.0:{port}")).is_err()
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

fn cargo_run_server() -> Result<(), std::io::Error> {
    let daemon = cargo_run_and_read("daemon", vec!["serve"])?;
    daemon.join().expect("unable to join thread")?;
    Ok(())
}

fn cargo_run_and_read(
    bin_name: &str,
    bin_args: Vec<&str>,
) -> Result<JoinHandle<Result<(), std::io::Error>>, std::io::Error> {
    let mut args = vec!["run", "--bin", bin_name, "--"];
    for bin_arg in bin_args {
        args.push(bin_arg);
    }
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

fn generate_cert_and_key_files(hostname: &str) -> Result<(), std::io::Error> {
    cmd!(
        "openssl",
        "req",
        "-x509",
        "-newkey",
        "rsa:4096",
        "-nodes",
        "-keyout",
        format!("assets/{hostname}-key.pem"),
        "-out",
        format!("assets/{hostname}-crt.pem"),
        "-subj",
        format!("/C=CA/ST=BC/L=Vancouver/O=Dis/CN={hostname}"),
    )
    .run()?;
    println!("generated a new cert for hostname {hostname}");
    Ok(())
}
