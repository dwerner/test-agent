use std::{
    env,
    path::{Path, PathBuf},
};

use duct::cmd;
use structopt::StructOpt;

use crate::common::BUILD_DIR;

const DEFAULT_BRANCH: &str = "dev";
const DEFAULT_REMOTE: &str = "origin";

const CASPER_NODE_GIT_REPO: &str = "https://github.com/casper-network/casper-node";
const CASPER_CLIENT_GIT_REPO: &str = "https://github.com/casper-ecosystem/casper-client-rs";
const CASPER_LAUNCHER_GIT_REPO: &str = "https://github.com/casper-network/casper-node-launcher";

#[derive(StructOpt, Debug)]
pub struct CheckoutAndCompileRustProject {
    /// Compile as debug (--release or not)
    #[structopt(short, long)]
    pub debug: bool,

    /// Git uri (http or git) to use for checkout
    #[structopt(short, long)]
    pub git_url: String,

    /// Branch name to use for checkout
    #[structopt(default_value = "dev")]
    pub branch: String,

    /// Name of the remote to use for checkouts
    #[structopt(default_value = "origin")]
    pub remote: String,

    /// Base dir where all checkouts are held
    #[structopt(default_value = BUILD_DIR)]
    pub base_path: PathBuf,

    /// Name for the local checkout
    #[structopt(short, long)]
    pub local_name: String,

    /// Name of the package to build - will use local_name if None.
    #[structopt(short, long)]
    pub package_name: Option<String>,
}

impl CheckoutAndCompileRustProject {
    /// Defaults for compiling the dev branch of the node repo.
    pub(crate) fn client_defaults() -> Self {
        Self {
            debug: false,
            git_url: CASPER_CLIENT_GIT_REPO.into(),
            branch: "main".into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_name: "casper-client".into(),
            package_name: None,
        }
    }
    /// Defaults for compiling the dev branch of the node repo.
    pub(crate) fn node_defaults() -> Self {
        Self {
            debug: false,
            git_url: CASPER_NODE_GIT_REPO.into(),
            branch: DEFAULT_BRANCH.into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_name: "casper-node".into(),
            package_name: None,
        }
    }
    /// Defaults for compiling the dev branch of the global-state-update-gen tool.
    pub(crate) fn global_state_update_gen_defaults() -> Self {
        Self {
            debug: false,
            git_url: CASPER_NODE_GIT_REPO.into(),
            branch: DEFAULT_BRANCH.into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_name: "casper-node".into(),
            package_name: Some("global-state-update-gen".into()),
        }
    }
    /// Defaults for compiling the dev branch of the launcher repo.
    pub(crate) fn launcher_defaults() -> Self {
        Self {
            debug: false,
            git_url: CASPER_LAUNCHER_GIT_REPO.into(),
            branch: "master".into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_name: "casper-node-launcher".into(),
            package_name: None,
        }
    }
}

/// Compile all projects with default settings.
pub fn compile_all_projects_in_separate_threads() -> Result<(), anyhow::Error> {
    checkout_and_compile(CheckoutAndCompileRustProject::client_defaults())?;
    checkout_and_compile(CheckoutAndCompileRustProject::node_defaults())?;
    // global state update gen is in the node repo, and depends on a checkout
    checkout_and_compile(CheckoutAndCompileRustProject::global_state_update_gen_defaults())?;
    checkout_and_compile(CheckoutAndCompileRustProject::launcher_defaults())?;
    Ok(())
}

// (Optionally) git checkout and compile project
// Not thread safe as we change dirs
pub fn checkout_and_compile(
    CheckoutAndCompileRustProject {
        debug,
        git_url,
        branch,
        remote,
        base_path,
        local_name,
        package_name,
    }: CheckoutAndCompileRustProject,
) -> Result<(), anyhow::Error> {
    let target_path = base_path.join(&local_name);
    println!("checking for local checkout");
    if !Path::new(&target_path).exists() {
        println!("checking out repo in {}", target_path.display());
        cmd!("git", "clone", git_url, &target_path).run()?;
    } else {
        println!("found checkout in {}", target_path.display());
    }

    // TODO: injection of rustflags, capture of output
    println!("compiling project {local_name} {package_name:?}");

    let starting_dir = std::env::current_dir()?;
    env::set_current_dir(&target_path)?;

    // fetching and switching branches supports an existing checkout
    println!("updating repo - fetching remote: {remote}");
    cmd!("git", "fetch", remote).run()?;

    println!(
        "checking out target branch {branch} in {}",
        target_path.display()
    );
    cmd!("git", "checkout", branch).run()?;

    let pkg = package_name.unwrap_or(local_name);
    if debug {
        cmd!("cargo", "build", "--package", pkg).run()?;
    } else {
        cmd!("cargo", "build", "--package", pkg, "--release").run()?;
    }
    env::set_current_dir(starting_dir)?;
    Ok(())
}
