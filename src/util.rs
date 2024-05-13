use std::fs::File;
use std::path::{Path, PathBuf};

use ron::de;

use crate::config::{Config, ParsedConfig};

const CONFIG_FILE_NAME: &str = "amdgpu-profile-switcher.ron";

const SEARCH_CONFIG_DIRS: &[&str] = &[
    "/etc/",
    "/etc/xdg/",
];

pub fn config_path() -> Option<PathBuf> {
    use std::env;
    use std::path::PathBuf;

    if let Ok(s) = env::var("APS_CONFIG_PATH") {
        return Some(PathBuf::from(s));
    }

    if let Ok(paths) = env::var("XDG_CONFIG_DIRS") {
        let path = env::split_paths(&paths)
            .map(|p| p.join(CONFIG_FILE_NAME))
            .find(|p| p.exists());

        if path.is_some() {
            return path;
        }
    }

    SEARCH_CONFIG_DIRS
        .iter()
        .map(|s| PathBuf::from(s).join(CONFIG_FILE_NAME))
        .find(|path| path.exists())
}

pub fn load_config(config_path: &Path) -> ParsedConfig {
    let f = File::open(config_path).unwrap();

    let pre_config: Config = match de::from_reader(f) {
        Ok(v) => v,
        Err(e) => {
            println!("{e:?}");
            panic!();
        },
    };
    pre_config.parse()
}
