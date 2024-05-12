use std::fs::File;
use std::path::{Path, PathBuf};

use ron::de;

use crate::config::{Config, ParsedConfig};

const CONFIG_FILE_NAME: &str = "amdgpu-profile-switcher.ron";

pub fn config_path() -> PathBuf {
    use std::env;
    use std::path::PathBuf;

    env::var("APS_CONFIG_PATH").ok().map(|s| PathBuf::from(s)).unwrap_or_else(|| {
        let config_home = env::var("XDG_CONFIG_HOME").unwrap_or("./".to_string());
        PathBuf::from(config_home).join(CONFIG_FILE_NAME)
    })
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
