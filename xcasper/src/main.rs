use std::path::Path;

use duct::cmd;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(subcommand)]
    command: Option<Command>,
}

const BUILD_DIR: &str = "xcasper-checkout";
const CASPER_NODE_GIT_REPO: &str = "https://github.com/casper-network/casper-node";
const CASPER_CLIENT_GIT_REPO: &str = "https://github.com/casper-ecosystem/casper-client-rs";
const CASPER_LAUNCHER_GIT_REPO: &str = "https://github.com/casper-network/casper-node-launcher";

#[derive(StructOpt, Debug)]
struct CompileRustProject {
    #[structopt(short, long)]
    debug: bool,
    #[structopt(default_value = "dev")]
    branch: String,
    #[structopt(default_value = "origin")]
    remote: String,
    #[structopt(default_value = "xcasper-checkout/casper-node")]
    checkout_path: String,
}

#[derive(StructOpt, Debug)]
enum Command {
    GenerateNetworkAssets {
        network_name: String,
    },
    CompileNode(CompileRustProject),
    CompileLauncher(CompileRustProject),
    StageUpgrade,
    LoadTest,
}

impl Command {
    fn dispatch(self) -> anyhow::Result<()> {
        println!("xtask : {:?}", self);
        match self {
            Command::GenerateNetworkAssets { network_name: _ } => todo!(),
            Command::CompileNode(CompileRustProject { debug, branch, remote, checkout_path }) => {
                compile_node(checkout_path, remote, branch, debug)
            }
            Command::CompileLauncher(CompileRustProject { debug, branch, remote, checkout_path }) => { todo!() }
            Command::StageUpgrade => todo!(),
            Command::LoadTest => todo!(),
        }
    }
}

fn compile_node(checkout_path: String, remote: String, branch: String, debug: bool) -> Result<(), anyhow::Error> {
    println!("checking for local checkout");
    if !Path::new(&checkout_path).exists() {
        println!("checking out node repo");
        cmd!("git", "clone", CASPER_NODE_GIT_REPO, &checkout_path).run()?;
    } else {
        println!("found checkout in {checkout_path}");
    }

    // TODO: injection of rustflags, capture of output
    println!("compiling casper-node");

    let starting_dir = std::env::current_dir()?;
    let mut checkout_dir = starting_dir.clone();
    checkout_dir.push(&checkout_path);
    std::env::set_current_dir(checkout_dir)?;

    println!("updating repo - fetching remote: {remote}");
    cmd!("git", "fetch", remote).run()?;

    println!("checkout out target branch {branch}");
    cmd!("git", "checkout", branch).run()?;
    if debug {
        cmd!("cargo", "build", "--package", "casper-node").run()?;
    } else {
        cmd!("cargo", "build", "--package", "casper-node", "--release").run()?;
    }
    std::env::set_current_dir(starting_dir)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    match args.command {
        Some(command) => {
            command.dispatch()?;
        }
        _ => {}
    }
    Ok(())
}
