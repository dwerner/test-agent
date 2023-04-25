mod assets;
mod common;
mod compile;

use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::Deserialize;
use structopt::StructOpt;

use assets::{generate_network_config_assets, GenerateNetworkAssets};
use compile::{BuildArtifacts, BuildProject, CargoBuildRustProject, CheckoutGitRepo};

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(subcommand)]
    command: Option<Command>,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Compile a single rust project, optionally from an existing checkout, otherwise checkout the project and compile
    ///
    /// Example:
    /// ```bash
    /// cargo xcasper compile node /path/to/casper-node
    /// ```
    /// or
    ///
    /// Check out the client and compile it with debug=true and default options
    /// ```bash
    /// cargo xcasper compile -d client
    /// ```
    ///
    Compile(Compile),

    /// Compile all projects
    CompileAllProjects {
        #[structopt(short, long)]
        config: Option<PathBuf>,
    },

    /// Copy artifacts to network directory, can be optionally specified with an artifacts.yaml file
    CopyArtifactsToNetworkDir(CopyArtifactsToNetworkDir),

    /// Generate network assets, config only
    /// Will generate the assets folder and the config files
    GenNetworkConfig(GenerateNetworkAssets),

    /// Stage an upgrade
    StageUpgrade,
}

#[derive(Debug, Deserialize)]
struct CompileYaml {
    node: Compile,
    client: Compile,
    db_utils: Compile,
    launcher: Compile,
    global_state_update_gen: Compile,
}

#[derive(StructOpt, Debug, Deserialize)]
struct CopyArtifactsToNetworkDir {
    /// Path to the network directory
    target_network_dir: Option<PathBuf>,

    #[structopt(default_value = "xcasper-staging/casper-node/target/release:^casper-node$")]
    node: BuildArtifacts,

    #[structopt(default_value = "xcasper-staging/casper-client/target/release:^casper-client$")]
    client: BuildArtifacts,

    #[structopt(
        default_value = "xcasper-staging/casper-node-launcher/target/release:^casper-node-launcher$"
    )]
    launcher: BuildArtifacts,

    #[structopt(
        default_value = "xcasper-staging/casper-db-utils/target/release:^casper-db-utils$"
    )]
    db_utils: BuildArtifacts,

    #[structopt(
        default_value = "xcasper-staging/casper-node/target/wasm32-unknown-unknown/release:.*\\.wasm$"
    )]
    contracts: BuildArtifacts,

    #[structopt(
        default_value = "xcasper-staging/casper-node/target/release:^global-state-update-gen$"
    )]
    global_state_update_gen: BuildArtifacts,
}

impl Command {
    /// Dispatch the command
    fn dispatch(self) -> Result<(), anyhow::Error> {
        match self {
            Command::CompileAllProjects {
                config: Some(config_yaml),
            } => {
                println!("using compile.yaml values");
                let reader = BufReader::new(File::open(config_yaml)?);
                let CompileYaml {
                    node,
                    client,
                    db_utils,
                    launcher,
                    global_state_update_gen,
                } = serde_yaml::from_reader(reader)?;
                for compile in [node, client, db_utils, launcher, global_state_update_gen] {
                    let artifacts = compile.dispatch()?;
                    println!(
                        "Compiled project, artifacts in {}",
                        artifacts.path.display()
                    );
                }
            }
            Command::CompileAllProjects { config: None } => {
                for project in [
                    Project::Node,
                    Project::Client,
                    Project::DbUtils,
                    Project::GlobalStateUpdateGen,
                    Project::Launcher,
                ] {
                    let artifacts = Compile {
                        project,
                        existing_checkout: None,
                        debug: false,
                    }
                    .dispatch()?;
                    println!(
                        "Compiled project, artifacts in {}",
                        artifacts.path.display()
                    );
                }
            }
            Command::Compile(compile) => {
                let artifacts = compile.dispatch()?;
                println!(
                    "Compiled project, artifacts in {}",
                    artifacts.path.display()
                );
            }
            Command::GenNetworkConfig(generate) => {
                let artifacts = generate_network_config_assets(generate)?;
                println!(
                    "Generated network config assets at {}",
                    artifacts.path.display()
                );
            }
            Command::CopyArtifactsToNetworkDir(CopyArtifactsToNetworkDir {
                target_network_dir,
                node,
                client,
                launcher,
                db_utils,
                contracts,
                global_state_update_gen,
            }) => {
                let target_network_dir = match target_network_dir {
                    Some(target_network_dir) => target_network_dir,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Target network directory must be specified"
                        ));
                    }
                };
                // ensure the target network directory exists
                if !target_network_dir.exists() {
                    return Err(anyhow::anyhow!(
                        "Target network directory does not exist at {}, have config files been generated yet?",
                        target_network_dir.display()
                    ));
                }

                let target_network_dir = target_network_dir.canonicalize()?;
                let target_network_shared_dir = target_network_dir.join("shared");

                // ensure the target network directory has the required subdirectoriess
                let target_bin_dir = target_network_shared_dir.join("bin");
                if !target_bin_dir.exists() {
                    std::fs::create_dir(&target_bin_dir)?;
                }
                let target_contracts_dir = target_network_shared_dir.join("contracts");
                if !target_contracts_dir.exists() {
                    std::fs::create_dir(&target_contracts_dir)?;
                }

                // ensure the binaries exist
                let bins = [
                    ("node", node),
                    ("client", client),
                    ("launcher", launcher),
                    ("db_utils", db_utils),
                    ("global_state_update_gen", global_state_update_gen),
                ];

                for (bin_name, bin) in bins.iter() {
                    if !bin.files_exist() {
                        return Err(anyhow::anyhow!(
                            "Binary {} does not exist at {}",
                            bin_name,
                            bin.path.display()
                        ));
                    }
                }

                // ensure the contracts exist
                if !contracts.files_exist() {
                    return Err(anyhow::anyhow!(
                        "Contracts do not exist at {}, have they been compiled yet? {:?}",
                        contracts.path.display(),
                        contracts,
                    ));
                };

                // actually copy the files
                for bin in bins.iter().map(|(_, bin)| bin) {
                    bin.copy_files_to(&target_bin_dir)?;
                }
                contracts.copy_files_to(&target_contracts_dir)?;
            }
            Command::StageUpgrade => todo!(),
        }
        Ok(())
    }
}

/// List of rust projects belonging to the deployment
#[derive(StructOpt, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    /// Build rust smart contracts (build-contracts-rs) using the Makefile in casper-node
    MakefileBuildContractsRs,
}

impl FromStr for Project {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "casper-db-utils" => Ok(Project::DbUtils),
            "casper-node" => Ok(Project::Node),
            "casper-client" => Ok(Project::Client),
            "casper-node-launcher" => Ok(Project::Launcher),
            "global-state-update-gen" => Ok(Project::GlobalStateUpdateGen),
            "makefile-build-contracts" => Ok(Project::MakefileBuildContractsRs),
            _ => Err(anyhow::anyhow!("Invalid project name. Must be one of: casper-db-utils, casper-node, casper-client, casper-node-launcher, global-state-update-gen, makefile-build-contracts")),
        }
    }
}

#[derive(StructOpt, Debug, Deserialize)]
struct Compile {
    /// Short name of project to compile
    project: Project,

    /// If specified, use this checkout instead of checking out the project
    existing_checkout: Option<PathBuf>,

    /// Compile as debug (--release or not)
    #[structopt(short, long)]
    debug: bool,
}

impl Compile {
    fn dispatch(self) -> Result<BuildArtifacts, anyhow::Error> {
        self.compile_options()?.dispatch()
    }

    /// Configure to compile the project. If there an existing checkout has been specified, use that, otherwise checkout the project.
    fn compile_options(&self) -> Result<BuildProject, anyhow::Error> {
        let checkout = match self.existing_checkout {
            Some(ref path) => path.clone(),
            None => match self.project {
                Project::DbUtils => CheckoutGitRepo::db_utils_defaults().dispatch()?,
                Project::Client => CheckoutGitRepo::client_defaults().dispatch()?,
                Project::MakefileBuildContractsRs | Project::Node => {
                    CheckoutGitRepo::node_defaults().dispatch()?
                }
                Project::Launcher => {
                    CheckoutGitRepo::global_state_update_gen_defaults().dispatch()?
                }
                Project::GlobalStateUpdateGen => CheckoutGitRepo::launcher_defaults().dispatch()?,
            },
        };

        let compile = match self.project {
            Project::DbUtils => BuildProject::Cargo(CargoBuildRustProject::new(
                checkout,
                "casper-db-utils",
                self.debug,
            )),
            Project::Client => BuildProject::Cargo(CargoBuildRustProject::new(
                checkout,
                "casper-client",
                self.debug,
            )),
            Project::Node => BuildProject::Cargo(CargoBuildRustProject::new(
                checkout,
                "casper-node",
                self.debug,
            )),
            Project::Launcher => BuildProject::Cargo(CargoBuildRustProject::new(
                checkout,
                "global-state-update-gen",
                self.debug,
            )),
            Project::GlobalStateUpdateGen => BuildProject::Cargo(CargoBuildRustProject::new(
                checkout,
                "casper-node-launcher",
                self.debug,
            )),
            Project::MakefileBuildContractsRs => BuildProject::Make {
                makefile_root: checkout.clone(),
                target: "build-contracts-rs".to_string(),
                build_dir: checkout
                    .join("target")
                    .join("wasm32-unknown-unknown")
                    .join("release"),
                artifact_suffix: ".wasm".to_string(),
            },
        };

        Ok(compile)
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();

    match args.command {
        Some(Command::CopyArtifactsToNetworkDir(command)) => {
            // if the artifacts.yaml file exists, use that instead of the command line args
            let artifacts_yaml = Path::new("artifacts.yaml");
            let command = if artifacts_yaml.exists() {
                println!("using artifacts.yaml values");
                let reader = BufReader::new(File::open(artifacts_yaml)?);
                serde_yaml::from_reader(reader)?
            } else {
                command
            };
            Command::CopyArtifactsToNetworkDir(command).dispatch()?;
        }
        Some(command) => {
            command.dispatch()?;
        }
        _ => {
            println!("no command given")
        }
    }
    Ok(())
}
