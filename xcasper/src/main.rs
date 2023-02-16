use std::path::Path;

use duct::cmd;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(subcommand)]
    command: Option<Command>,
}

const BUILD_DIR: &str = "xcasper-build";
const CASPER_NODE_GIT_REPO: &str = "https://github.com/casper-network/casper-node";
const CASPER_CLIENT_GIT_REPO: &str = "https://github.com/casper-ecosystem/casper-client-rs";
const CASPER_LAUNCHER_GIT_REPO: &str = "https://github.com/casper-network/casper-node-launcher";

#[derive(StructOpt, Debug)]
enum Command {
    GenerateNetworkAssets {
        network_name: String,
    },
    CompileNode {
        #[structopt(short, long)]
        debug: bool,
    },
    CompileLauncher {
        #[structopt(short, long)]
        debug: bool,
    },
    StageUpgrade,
    LoadTest,
}

impl Command {
    fn dispatch(self) -> anyhow::Result<()> {
        println!("xtask : {:?}", self);
        //let deps = built_info;
        match self {
            Command::GenerateNetworkAssets { network_name } => todo!(),
            Command::CompileNode { debug } => {
                println!("checking for local checkout of casper-node");
                let node_path = format!("{BUILD_DIR}/casper-node");
                if !Path::new(&node_path).exists() {
                    git2::Repository::clone(CASPER_NODE_GIT_REPO, node_path)?;
                }
                // TODO: injection of rustflags, capture of output
                cmd!("cd {BUILD_DIR}/casper-node").run()?;
                if debug {
                    cmd!("cargo", "build", "--package", "casper-node").run()?;
                } else {
                    cmd!("cargo", "build", "--package", "casper-node", "--release").run()?;
                }
                Ok(())
            }
            Command::CompileLauncher { debug } => todo!(),
            Command::StageUpgrade => todo!(),
            Command::LoadTest => todo!(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    match args.command {
        Some(command) => {
            command.dispatch();
        }
        _ => {}
    }
    Ok(())
}
