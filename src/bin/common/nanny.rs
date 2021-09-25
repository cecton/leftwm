//! Starts programs in autostart, runs global 'up' script, and boots theme. Provides function to
//! boot other desktop files also.
use anyhow::Result;
use leftwm::utils::child_process::Children;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use xdg::BaseDirectories;

#[derive(Default)]
pub struct Nanny;

impl Nanny {
    #[must_use]
    #[allow(dead_code)]
    pub fn autostart() -> Children {
        dirs_next::home_dir()
            .map(|mut path| {
                path.push(".config");
                path.push("autostart");
                path
            })
            .and_then(|path| list_desktop_files(&path).ok())
            .map(|files| {
                files
                    .iter()
                    .filter_map(|file| boot_desktop_file(file).ok())
                    .collect::<Children>()
            })
            .unwrap_or_default()
    }

    /// Retrieve the path to the config directory. Tries to create it if it does not exist.
    ///
    /// # Errors
    ///
    /// Will error if unable to open or create the config directory.
    /// Could be caused by inadequate permissions.
    fn get_config_dir() -> Result<PathBuf> {
        BaseDirectories::with_prefix("leftwm")?
            .create_config_directory("")
            .map_err(|e| e.into())
    }

    /// Runs a script if it exits
    fn run_script(path: &Path) -> Result<Option<Child>> {
        if path.is_file() {
            Command::new(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .spawn()
                .map(Some)
                .map_err(|e| e.into())
        } else {
            Ok(None)
        }
    }

    /// Runs the 'up' script in the config directory, if there is one.
    ///
    /// # Errors
    ///
    /// Will error if unable to open current config directory.
    /// Could be caused by inadequate permissions.
    pub fn run_global_up_script() -> Result<Option<Child>> {
        let mut path = Self::get_config_dir()?;
        path.push("up");
        Self::run_script(&path)
    }

    /// Runs the 'up' script of the current theme, if there is one.
    ///
    /// # Errors
    ///
    /// Will error if unable to open current theme directory.
    /// Could be caused by inadequate permissions.
    pub fn boot_current_theme() -> Result<Option<Child>> {
        let mut path = Self::get_config_dir()?;
        path.push("themes");
        path.push("current");
        path.push("up");
        Self::run_script(&path)
    }
}

#[allow(dead_code)]
fn boot_desktop_file(path: &Path) -> std::io::Result<Child> {
    let args = format!( "`if [ \"$(grep '^X-GNOME-Autostart-enabled' {:?} | tail -1 | sed 's/^X-GNOME-Autostart-enabled=//' | tr '[A-Z]' '[a-z]')\" != 'false' ]; then grep '^Exec' {:?} | tail -1 | sed 's/^Exec=//' | sed 's/%.//' | sed 's/^\"//g' | sed 's/\" *$//g'; else echo 'exit'; fi`", path , path);
    Command::new("sh").arg("-c").arg(args).spawn()
}

// get all the .desktop files in a folder
#[allow(dead_code)]
fn list_desktop_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut list = vec![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "desktop" {
                        list.push(path);
                    }
                }
            }
        }
    }
    Ok(list)
}
