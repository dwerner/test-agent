use std::io::BufReader;

use duct::{cmd, ReaderHandle};

pub trait PkgManager {
    //must return the exit code
    fn is_installed(&mut self, pkg: String) -> bool;
    fn install(&mut self, pkg: String) -> Result<BufReader<ReaderHandle>, std::io::Error>;
    fn uninstall(&mut self, pkg: String) -> Result<BufReader<ReaderHandle>, std::io::Error>;
    fn no_confirm(&mut self, set: bool);
    fn get_name(&self) -> String;
}

#[derive(Debug)]
pub enum PkgWrapperError {
    NoSupportedPackageManagerFound,
    UserIsNotRoot,
}

pub struct PkgWrapper {
    pkg_manager: Box<dyn PkgManager>,
}

impl PkgWrapper {
    pub fn new(no_confirm: bool) -> Result<Self, PkgWrapperError> {
        // if unsafe { libc::getuid() } != 0 {
        //     return Err(PkgWrapperError::UserIsNotRoot);
        // }
        let vec: Vec<Box<dyn PkgManager>> = vec![Box::new(Pacman::new()), Box::new(Apt::new())];
        for mut pkg in vec {
            if std::path::Path::new(&format!("/bin/{}", pkg.get_name())).exists() {
                pkg.no_confirm(no_confirm);
                return Ok(Self { pkg_manager: pkg });
            }
        }
        Err(PkgWrapperError::NoSupportedPackageManagerFound)
    }
    pub fn with_custom_pkg_managers(
        no_confirm: bool,
        vec: Vec<Box<dyn PkgManager>>,
    ) -> Result<Self, PkgWrapperError> {
        for mut pkg in vec {
            if std::path::Path::new(&format!("/bin/{}", pkg.get_name())).exists() {
                pkg.no_confirm(no_confirm);
                return Ok(Self { pkg_manager: pkg });
            }
        }
        Err(PkgWrapperError::NoSupportedPackageManagerFound)
    }

    pub fn is_installed<T: ToString>(&mut self, pkg: T) -> bool {
        self.pkg_manager.is_installed(pkg.to_string())
    }

    pub fn install_pkg<T: ToString>(
        &mut self,
        pkg: T,
    ) -> Result<BufReader<ReaderHandle>, std::io::Error> {
        self.pkg_manager.install(pkg.to_string())
    }

    pub fn uninstall_pkg<T: ToString>(
        &mut self,
        pkg: T,
    ) -> Result<BufReader<ReaderHandle>, std::io::Error> {
        self.pkg_manager.uninstall(pkg.to_string())
    }
}

pub struct Apt {
    no_confirm: bool,
}

impl Apt {
    pub fn new() -> Self {
        Self { no_confirm: false }
    }
}

impl PkgManager for Apt {
    fn install(&mut self, pkg: String) -> Result<BufReader<ReaderHandle>, std::io::Error> {
        let args = if self.no_confirm {
            vec!["remove".into(), pkg, "-y".into()]
        } else {
            vec!["remove".into(), pkg]
        };
        let cmd = BufReader::new(duct::cmd("apt", args).reader()?);
        Ok(cmd)
    }
    fn uninstall(&mut self, pkg: String) -> Result<BufReader<ReaderHandle>, std::io::Error> {
        let args = if self.no_confirm {
            vec!["install".into(), pkg, "-y".into()]
        } else {
            vec!["install".into(), pkg]
        };
        let cmd = BufReader::new(duct::cmd("apt", args).reader()?);
        Ok(cmd)
    }
    fn no_confirm(&mut self, set: bool) {
        self.no_confirm = set;
    }
    fn get_name(&self) -> String {
        "apt".into()
    }

    fn is_installed(&mut self, pkg: String) -> bool {
        cmd!("dpkg", "-l", pkg).reader().is_ok()
    }
}

pub struct Pacman {
    no_confirm: bool,
}

impl Pacman {
    pub fn new() -> Self {
        Self { no_confirm: false }
    }
}

impl PkgManager for Pacman {
    fn uninstall(&mut self, pkg: String) -> Result<BufReader<ReaderHandle>, std::io::Error> {
        let args = if self.no_confirm {
            vec!["-Rns".into(), pkg, "--noconfirm".into()]
        } else {
            vec!["-Rns".into(), pkg]
        };
        let cmd = BufReader::new(duct::cmd("pacman", args).reader()?);
        Ok(cmd)
    }
    fn install(&mut self, pkg: String) -> Result<BufReader<ReaderHandle>, std::io::Error> {
        let args = if self.no_confirm {
            vec!["-Sy".into(), pkg, "--noconfirm".into()]
        } else {
            vec!["-Sy".into(), pkg]
        };
        let cmd = BufReader::new(duct::cmd("pacman", args).reader()?);
        Ok(cmd)
    }
    fn no_confirm(&mut self, set: bool) {
        self.no_confirm = set;
    }
    fn get_name(&self) -> String {
        "pacman".into()
    }

    fn is_installed(&mut self, pkg: String) -> bool {
        cmd!("pacman", "-Q", &pkg).reader().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::PkgWrapper;

    #[test]
    fn detect() {
        PkgWrapper::new(true).unwrap();
    }
    #[test]
    fn install() {
        PkgWrapper::new(true).unwrap().install_pkg("htop").ok();
    }
}
