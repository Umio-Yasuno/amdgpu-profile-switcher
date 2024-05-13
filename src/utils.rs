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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs;

pub fn watch_config_file(config_path: &Path) -> Arc<AtomicBool> {
    let config_path = config_path.to_path_buf();
    let is_modified = Arc::new(AtomicBool::new(false));
    let arc_is_modified = is_modified.clone();
    let metadata = fs::metadata(&config_path).unwrap_or_else(|e| panic!("Error: {e}"));
    // https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.modified
    let systime = metadata.modified().expect("Not supported on this platform");

    std::thread::spawn(move || {
        let mut pre_systime = systime;

        loop {
            if let Ok(systime) = fs::metadata(&config_path).and_then(|meta| meta.modified()) {
                if pre_systime < systime {
                    pre_systime = systime;
                    arc_is_modified.store(true, Ordering::Release);
                }
            }
            
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    is_modified
}
