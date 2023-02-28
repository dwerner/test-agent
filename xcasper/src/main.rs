mod assets;
mod common;
mod compile;

use structopt::StructOpt;

use assets::{generate_network_assets, GenerateNetworkAssets};
use compile::{checkout_and_compile, compile_all_projects, CheckoutAndCompileRustProject};

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(StructOpt, Debug)]
enum Command {
    GenNetwork(GenerateNetworkAssets),
    CompileAllProjects,
    CompileNodeWithDefaults,
    CompileClientWithDefaults,
    CompileLauncherWithDefaults,
    CompileGlobalStateUpdateGenWithDefaults,
    CompileProject(CheckoutAndCompileRustProject),
    StageUpgrade,
    LoadTest,
}

impl Command {
    fn dispatch(self) -> anyhow::Result<()> {
        println!("xcasper : {:?}", self);
        match self {
            Command::GenNetwork(generate) => generate_network_assets(generate),
            Command::CompileAllProjects => compile_all_projects(),
            Command::CompileClientWithDefaults => {
                checkout_and_compile(CheckoutAndCompileRustProject::client_defaults())
            }
            Command::CompileNodeWithDefaults => {
                checkout_and_compile(CheckoutAndCompileRustProject::node_defaults())
            }
            Command::CompileLauncherWithDefaults => {
                checkout_and_compile(CheckoutAndCompileRustProject::launcher_defaults())
            }
            Command::CompileGlobalStateUpdateGenWithDefaults => checkout_and_compile(
                CheckoutAndCompileRustProject::global_state_update_gen_defaults(),
            ),
            Command::CompileProject(compile) => checkout_and_compile(compile),
            Command::StageUpgrade => todo!(),
            Command::LoadTest => todo!(),
        }
    }
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
