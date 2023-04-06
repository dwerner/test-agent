mod assets;
mod common;
mod compile;

use std::{path::PathBuf, str::FromStr};

use structopt::StructOpt;

use assets::{generate_network_assets, GenerateNetworkAssets};
use compile::{CheckoutGitRepo, CompileRustProject};

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Generate network assets
    GenNetwork(GenerateNetworkAssets),
    /// Compile a rust project, optionally from an existing checkout, otherwise checkout the project and compile
    Compile(Compile),
    /// Stage an upgrade
    StageUpgrade,
}

impl Command {
    /// Dispatch the command
    fn dispatch(self) -> Result<PathBuf, anyhow::Error> {
        println!("xcasper : {:?}", self);
        match self {
            Command::GenNetwork(generate) => generate_network_assets(generate),
            Command::Compile(compile) => compile.dispatch(),
            Command::StageUpgrade => todo!(),
        }
    }
}

/// List of rust projects belonging to the deployment
#[derive(StructOpt, Debug)]
enum Project {
    /// Compile casper-db-utils
    DbUtils,
    /// Compile casper-client
    Node,
    /// Compile casper-node
    Client,
    /// Compile casper-node-launcher
    Launcher,
    /// Compile global-state-update-gen
    GlobalStateUpdateGen,
}

impl FromStr for Project {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "db-utils" => Ok(Project::DbUtils),
            "node" => Ok(Project::Node),
            "client" => Ok(Project::Client),
            "launcher" => Ok(Project::Launcher),
            "global-state-update-gen" => Ok(Project::GlobalStateUpdateGen),
            _ => Err(anyhow::anyhow!("Invalid project name")),
        }
    }
}

#[derive(StructOpt, Debug)]
struct Compile {
    project: Project,
    existing_checkout: Option<PathBuf>,
}

impl Compile {
    fn dispatch(self) -> Result<PathBuf, anyhow::Error> {
        self.compile_options()?.dispatch()
    }

    /// If there an existing checkout has been specified, use that, otherwise checkout the project
    fn compile_options(&self) -> Result<CompileRustProject, anyhow::Error> {
        let checkout = match self.existing_checkout {
            Some(ref path) => path.clone(),
            None => match self.project {
                Project::DbUtils => CheckoutGitRepo::db_utils_defaults().dispatch()?,
                Project::Client => CheckoutGitRepo::client_defaults().dispatch()?,
                Project::Node => CheckoutGitRepo::node_defaults().dispatch()?,
                Project::Launcher => {
                    CheckoutGitRepo::global_state_update_gen_defaults().dispatch()?
                }
                Project::GlobalStateUpdateGen => CheckoutGitRepo::launcher_defaults().dispatch()?,
            },
        };

        let compile = match self.project {
            Project::DbUtils => CompileRustProject::new(checkout, "casper-db-utils"),
            Project::Client => CompileRustProject::new(checkout, "casper-client"),
            Project::Node => CompileRustProject::new(checkout, "casper-node"),
            Project::Launcher => CompileRustProject::new(checkout, "global-state-update-gen"),
            Project::GlobalStateUpdateGen => {
                CompileRustProject::new(checkout, "casper-node-launcher")
            }
        };

        Ok(compile)
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    match args.command {
        Some(command) => {
            command.dispatch()?;
        }
        _ => {
            println!("no command given")
        }
    }
    Ok(())
}
