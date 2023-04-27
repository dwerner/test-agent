use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use duct::cmd;
use regex::Regex;
use serde::{de, Deserialize, Deserializer};
use structopt::StructOpt;
use walkdir::WalkDir;

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
            branch: "dev".into(),
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
        env::set_current_dir(target_path)?;
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
pub struct CargoBuildRustProject {
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

#[derive(StructOpt, Debug)]
pub enum BuildProject {
    Make {
        makefile_root: PathBuf,
        target: String,
        build_dir: PathBuf,
        artifact_suffix: String,
    },
    Cargo(CargoBuildRustProject),
}

#[derive(Debug)]
pub struct BuildArtifacts {
    pub path: PathBuf,
    pub files: Vec<String>,
}

impl FromStr for BuildArtifacts {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');
        let path = parts.next().unwrap().into();
        BuildArtifacts::from_dir_with_regex(path, parts.next().unwrap())
    }
}

impl<'de> Deserialize<'de> for BuildArtifacts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

impl BuildArtifacts {
    pub fn new(path: PathBuf, files: Vec<String>) -> Self {
        Self { path, files }
    }

    pub fn from_dir_with_regex(path: PathBuf, regex: &str) -> Result<Self, anyhow::Error> {
        let re = Regex::new(regex)?;
        let mut files = vec![];
        for entry in WalkDir::new(&path).max_depth(1) {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if re.is_match(&file_name) {
                files.push(file_name);
            }
        }
        Ok(Self::new(path, files))
    }

    pub fn files_exist(&self) -> bool {
        if self.files.is_empty() {
            return false;
        }
        for file in &self.files {
            let file_path = self.path.join(file);
            if !file_path.exists() {
                println!("missing file: {}", file_path.display());
                return false;
            }
        }
        true
    }

    pub fn copy_files_to(&self, dest: &Path) -> Result<(), anyhow::Error> {
        if !self.files_exist() {
            return Err(anyhow::anyhow!(
                "Files do not exist, cannot copy to {}. Have they been built yet?",
                dest.display()
            ));
        }
        for file in &self.files {
            let file_path = self.path.join(file);
            let dest_path = dest.join(file);
            println!("Copying {} to {}", file_path.display(), dest_path.display());
            fs::copy(file_path, dest_path)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for BuildArtifacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BuildArtifact{{ path: {:?}, files: {} }}",
            self.path,
            self.files.len()
        )
    }
}

fn find_files_with_suffix_at_path(path: PathBuf, suffix: String) -> Vec<String> {
    let mut files = vec![];
    for entry in WalkDir::new(path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_file() && entry.path().to_string_lossy().ends_with(&suffix) {
            let file_stem = entry.path().file_stem().unwrap().to_string_lossy();
            let extension = entry.path().extension().unwrap().to_string_lossy();
            files.push(format!("{}.{}", file_stem, extension));
        }
    }
    files
}

// Supports building a project with either cargo or make
impl BuildProject {
    pub fn dispatch(self) -> Result<BuildArtifacts, anyhow::Error> {
        match self {
            BuildProject::Cargo(cargo_build_rust_project) => cargo_build_rust_project.dispatch(),
            BuildProject::Make {
                makefile_root,
                target,
                build_dir,
                artifact_suffix,
            } => {
                println!("compiling project with make at {:?}", makefile_root);
                let starting_dir = std::env::current_dir()?;
                env::set_current_dir(&makefile_root)?;
                cmd!("make", "-n", &target).run()?;
                env::set_current_dir(starting_dir)?;
                Ok(BuildArtifacts {
                    path: build_dir.clone(),
                    files: find_files_with_suffix_at_path(build_dir, artifact_suffix),
                })
            }
        }
    }
}

impl CargoBuildRustProject {
    pub fn new(target_path: PathBuf, package_name: &str, debug: bool) -> Self {
        Self {
            debug,
            package_name: package_name.into(),
            target_path,
        }
    }

    pub fn dispatch(self) -> Result<BuildArtifacts, anyhow::Error> {
        println!(
            "compiling project at {:?} {:?} Debug: {}",
            self.target_path, self.package_name, self.debug
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
        Ok(BuildArtifacts {
            path: self.target_path.join("target").join(if self.debug {
                "debug"
            } else {
                "release"
            }),
            files: vec![package_name],
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_build_artifacts_from_dir_with_regex() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::write(temp_path.join("file3.log"), "").unwrap();

        let build_artifacts =
            BuildArtifacts::from_dir_with_regex(temp_path.clone(), r".*\.txt$").unwrap();
        assert_eq!(build_artifacts.path, temp_path);
        assert_eq!(build_artifacts.files.len(), 2);
        assert!(build_artifacts.files.contains(&"file1.txt".to_string()));
        assert!(build_artifacts.files.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_build_artifacts_files_exist() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        fs::write(temp_path.join("file1.txt"), "").unwrap();
        fs::write(temp_path.join("file2.txt"), "").unwrap();
        fs::write(temp_path.join("file3.log"), "").unwrap();

        let build_artifacts =
            BuildArtifacts::from_dir_with_regex(temp_path.clone(), r".*\.txt$").unwrap();
        assert!(build_artifacts.files_exist());

        fs::remove_file(temp_path.join("file1.txt")).unwrap();
        assert!(!build_artifacts.files_exist());
    }

    #[test]
    fn test_find_files_with_suffix_at_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir_path = temp_dir.path();
        let file_path = temp_dir_path.join("test_file.txt");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"test").unwrap();
        let files = find_files_with_suffix_at_path(temp_dir_path.to_path_buf(), ".txt".into());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], "test_file.txt");
    }
}
