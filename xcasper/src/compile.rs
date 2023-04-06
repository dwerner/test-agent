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
const CASPER_DB_UTILS_REPO: &str = "https://github.com/casper-network/casper-db-utils";
const CASPER_LAUNCHER_GIT_REPO: &str = "https://github.com/casper-network/casper-node-launcher";

#[derive(StructOpt, Debug)]
pub struct CheckoutGitRepo {
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
    pub local_checkout_name: String,

    /// Should the checkout be updated from the remote
    #[structopt(short, long)]
    pub update_from_remote: bool,
}

impl CheckoutGitRepo {
    /// Defaults for compiling the dev branch of the casper-db-utils repo.
    pub(crate) fn db_utils_defaults() -> Self {
        Self {
            git_url: CASPER_DB_UTILS_REPO.into(),
            branch: "master".into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_checkout_name: "casper-db-utils".into(),
            update_from_remote: false,
        }
    }

    /// Defaults for compiling the dev branch of the node repo.
    pub(crate) fn client_defaults() -> Self {
        Self {
            git_url: CASPER_CLIENT_GIT_REPO.into(),
            branch: "main".into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_checkout_name: "casper-client".into(),
            update_from_remote: false,
        }
    }
    /// Defaults for compiling the dev branch of the node repo.
    pub(crate) fn node_defaults() -> Self {
        Self {
            git_url: CASPER_NODE_GIT_REPO.into(),
            branch: DEFAULT_BRANCH.into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_checkout_name: "casper-node".into(),
            update_from_remote: false,
        }
    }
    /// Defaults for compiling the dev branch of the global-state-update-gen tool.
    pub(crate) fn global_state_update_gen_defaults() -> Self {
        Self {
            git_url: CASPER_NODE_GIT_REPO.into(),
            branch: DEFAULT_BRANCH.into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_checkout_name: "casper-node".into(),
            update_from_remote: false,
        }
    }
    /// Defaults for compiling the dev branch of the launcher repo.
    pub(crate) fn launcher_defaults() -> Self {
        Self {
            git_url: CASPER_LAUNCHER_GIT_REPO.into(),
            branch: "main".into(),
            remote: DEFAULT_REMOTE.into(),
            base_path: BUILD_DIR.into(),
            local_checkout_name: "casper-node-launcher".into(),
            update_from_remote: false,
        }
    }

    // (Optionally) git checkout and compile project
    // Not thread safe as we change dirs
    pub fn dispatch(self) -> Result<PathBuf, anyhow::Error> {
        let target_path: &Path = &self.base_path.join(&self.local_checkout_name);
        println!("checking for local checkout");
        if !Path::new(&target_path).exists() {
            println!("checking out repo in {}", target_path.display());
            cmd!("git", "clone", self.git_url, &target_path).run()?;
        } else {
            println!("found checkout in {}", target_path.display());
        }
        let starting_dir = std::env::current_dir()?;
        env::set_current_dir(&target_path)?;
        println!("updating repo - fetching remote: {}", self.remote);
        cmd!("git", "fetch", &self.remote).run()?;
        println!(
            "checking out target branch {} in {}",
            self.remote,
            target_path.display()
        );
        cmd!("git", "checkout", &self.branch).run()?;
        if self.update_from_remote {
            cmd!("git", "pull", &self.remote, &self.branch).run()?;
        }
        env::set_current_dir(starting_dir)?;
        Ok(target_path.to_path_buf())
    }
}

#[derive(StructOpt, Debug)]
pub struct CompileRustProject {
    /// Compile as debug (--release or not)
    #[structopt(short, long)]
    pub debug: bool,

    /// Package name to compile.
    #[structopt(short, long)]
    pub package_name: String,

    /// Target path.
    #[structopt(short, long)]
    pub target_path: PathBuf,
}

impl CompileRustProject {
    pub fn new(target_path: PathBuf, package_name: &str) -> Self {
        Self {
            debug: false,
            package_name: package_name.into(),
            target_path,
        }
    }

    pub fn package(pkg: &str) -> impl FnOnce(Self) -> Self {
        let pkg = pkg.to_owned();
        move |proj| Self {
            package_name: pkg,
            ..proj
        }
    }

    pub fn debug() -> impl FnOnce(Self) -> Self {
        move |proj| Self {
            debug: true,
            ..proj
        }
    }

    pub fn dispatch(self) -> Result<PathBuf, anyhow::Error> {
        println!(
            "compiling project at {:?} {:?}",
            self.target_path, self.package_name
        );
        let starting_dir = std::env::current_dir()?;
        env::set_current_dir(&self.target_path)?;
        let package_name = self.package_name;
        if self.debug {
            cmd!("cargo", "build", "--package", &package_name).run()?;
        } else {
            cmd!("cargo", "build", "--package", &package_name, "--release").run()?;
        }
        env::set_current_dir(starting_dir)?;
        Ok(self
            .target_path
            .join("target")
            .join(if self.debug { "debug" } else { "release" })
            .join(package_name))
    }
}
