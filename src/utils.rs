use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::fs;

use ron::{de, ser};

use crate::{AMDGPU, AmdgpuDevice};
use crate::config::{Config, ConfigPerDevice, ConfigEntry, ParsedConfig, ParseConfigError};

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

const PERF_LEVEL_LIST: &[&str] = &[
    "auto",
    "low",
    "high",
    "manual",
    "profile_standard",
    "profile_peak",
    "profile_min_sclk",
    "profile_min_mclk",
    "perf_determinism",
];

const PROFILE_LIST: &[&str] = &[
    "BOOTUP_DEFAULT",
    "3D_FULL_SCREEN",
    "POWER_SAVING",
    "VIDEO",
    "VR",
    "COMPUTE",
    "CUSTOM",
    "WINDOW_3D",
    "CAPPED",
    "UNCAPPED",
];

pub fn load_raw_config(config_path: &Path) -> Config {
    let s = std::fs::read_to_string(config_path).unwrap();

    match de::from_str(&s) {
        Ok(v) => v,
        Err(e) => panic!("{e:?}"),
    }
}

pub fn load_config(config_path: &Path) -> ParsedConfig {
    let s = std::fs::read_to_string(config_path).unwrap();

    let config: Config = match de::from_str(&s) {
        Ok(v) => v,
        Err(e) => panic!("{e:?}"),
    };

    match config.parse() {
        Ok(v) => v,
        Err(e) => {
            let mut lines = s.lines().enumerate();
            let mut line_number: Option<usize> = None;

            match e {
                ParseConfigError::PciIsEmpty => {
                    line_number = lines
                        .find(|(_i, l)| l.replace(' ', "").contains("pci:\"\""))
                        .map(|(i, _l)| i);
                },
                ParseConfigError::EntryNameIsEmpty => {
                    line_number = lines
                        .find(|(_i, l)| l.replace(' ', "").contains("name:\"\""))
                        .map(|(i, _l)| i);
                },
                ParseConfigError::InvalidPerfLevel(ref invalid_perf_level) => {
                    eprintln!("`perf_level` must be one of the following: {PERF_LEVEL_LIST:?}");
                    line_number = lines
                        .find(|(_i, l)| l.contains(invalid_perf_level))
                        .map(|(i, _l)| i);
                },
                ParseConfigError::InvalidProfile(ref invalid_profile) => {
                    eprintln!("`profile` must be one of the following: {PROFILE_LIST:?}");
                    line_number = lines
                        .find(|(_i, l)| l.contains(invalid_profile))
                        .map(|(i, _l)| i);
                },
                _ => {},
            }

            if let Some(line_number) = line_number {
                panic!("Parse Error: {e:?}, Line {}", line_number+1);
            } else {
                panic!("Parse Error: {e:?}");
            }
        },
    }
}

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
            } else {
                // Maybe the config file is deleted or moved.
                // Handle the error on the main thread.
                arc_is_modified.store(true, Ordering::Release);
            }
            
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    is_modified
}

pub const COMMENT: &str = r#"// Config entries that are earlier take priority.
/*
    perf_level: [
        "auto",
        "low",
        "high",
        "manual",
        "profile_standard",
        "profile_peak",
        "profile_min_sclk",
        "profile_min_mclk",
        "perf_determinism",
    ],
    profile: [
        "BOOTUP_DEFAULT",
        "3D_FULL_SCREEN",
        "POWER_SAVING",
        "VIDEO",
        "VR",
        "COMPUTE",
        "CUSTOM",
        "WINDOW_3D",
        "CAPPED",
        "UNCAPPED",
    ],
*/
"#;

pub fn generate_config() -> ron::Result<String> {
    let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

    if pci_devs.is_empty() {
        panic!("No AMDGPU devices.");
    }

    let entry = ConfigEntry {
        name: "glxgears".to_string(),
        perf_level: None,
        profile: Some("BOOTUP_DEFAULT".to_string()),
        power_cap_watt: None,
        fan_target_temperature: None,
    };
    let config_devices: Vec<_> = pci_devs
        .iter()
        .filter_map(|pci| {
            let dev = AmdgpuDevice::get_from_pci_bus(*pci)?;

            Some(ConfigPerDevice {
                pci: pci.to_string(),
                _device_name: Some(dev.device_name),
                default_power_cap_watt: dev.power_cap.as_ref().map(|cap| cap.default),
                _min_power_cap_watt: dev.power_cap.as_ref().map(|cap| cap.min),
                _max_power_cap_watt: dev.power_cap.as_ref().map(|cap| cap.max),
                default_perf_level: None,
                default_profile: None,
                default_fan_target_temperature: None,
                entries: vec![entry.clone()],
            })
        })
        .collect();
    let config = Config { config_devices };

    ser::to_string_pretty(&config, Default::default())
}

pub fn save_config_file(config_path: &Path, config: &Config) -> std::io::Result<()> {
    let s = ser::to_string_pretty(&config, Default::default()).unwrap();

    fs::write(config_path, s)
}
