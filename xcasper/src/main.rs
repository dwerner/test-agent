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
    Compile(Compile),
    StageUpgrade,
    LoadTest,
}

impl Command {
    fn dispatch(self) -> anyhow::Result<()> {
        println!("xcasper : {:?}", self);
        match self {
            Command::GenNetwork(generate) => generate_network_assets(generate),
            Command::Compile(compile) => compile.dispatch(),
            Command::StageUpgrade => todo!(),
            Command::LoadTest => todo!(),
        }
    }
}

#[derive(StructOpt, Debug)]
enum Compile {
    All,
    DbUtils,
    Node,
    Client,
    Launcher,
    GlobalStateUpdateGen,
}

impl Compile {
    fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Compile::All => compile_all_projects(),
            Compile::DbUtils => {
                checkout_and_compile(CheckoutAndCompileRustProject::db_utils_defaults())
            }
            Compile::Client => {
                checkout_and_compile(CheckoutAndCompileRustProject::client_defaults())
            }
            Compile::Node => checkout_and_compile(CheckoutAndCompileRustProject::node_defaults()),
            Compile::Launcher => {
                checkout_and_compile(CheckoutAndCompileRustProject::launcher_defaults())
            }
            Compile::GlobalStateUpdateGen => checkout_and_compile(
                CheckoutAndCompileRustProject::global_state_update_gen_defaults(),
            ),
        }
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
