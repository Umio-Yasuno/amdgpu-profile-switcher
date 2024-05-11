use std::fs::File;
use std::path::{Path, PathBuf};

use libdrm_amdgpu_sys::AMDGPU;
use AMDGPU::{DpmForcedLevel, PowerProfile};

use proc_prog_name::ProcProgEntry;
use ron::de;
use log::debug;

mod config;
use config::{Config, ParsedConfig, ParsedConfigEntry};

mod amdgpu_device;
use amdgpu_device::AmdgpuDevice;

mod args;
use args::MainOpt;

struct AppDevice {
    pub amdgpu_device: AmdgpuDevice,
    pub config_device: config::ParsedConfigPerDevice,
}

impl AppDevice {
    fn set_perf_level(&self, perf_level: DpmForcedLevel) {
        let perf_level = perf_level.to_arg();
        std::fs::write(&self.amdgpu_device.dpm_perf_level_path, perf_level)
            .unwrap_or_else(|e| panic!("IO Error: {e}"));
    }

    fn reset_perf_level(&self) {
        let current = DpmForcedLevel::get_from_sysfs(&self.amdgpu_device.sysfs_path)
            .expect("Error: Failed to get current dpm force performance level.");
        match current {
            DpmForcedLevel::Auto |
            DpmForcedLevel::Manual => {},
            _ => {
                std::fs::write(&self.amdgpu_device.dpm_perf_level_path, DpmForcedLevel::Auto.to_arg())
                    .unwrap_or_else(|e| panic!("IO Error: {e}"));
            },
        }
    }

    fn set_power_profile(&self, profile: PowerProfile) {
        let profile = (profile as u32).to_string();
        std::fs::write(&self.amdgpu_device.power_profile_path, profile)
            .unwrap_or_else(|e| panic!("IO Error: {e}"));
    }

    fn reset_power_profile(&self) {
        let current_profile = PowerProfile::get_current_profile_from_sysfs(&self.amdgpu_device.sysfs_path)
            .expect("Error: Failed to get current power profile.");
        if current_profile != PowerProfile::BOOTUP_DEFAULT {
            let profile = (PowerProfile::BOOTUP_DEFAULT as u32).to_string();
            std::fs::write(&self.amdgpu_device.power_profile_path, profile)
                .unwrap_or_else(|e| panic!("IO Error: {e}"));
        }
    }
}

const CONFIG_FILE: &str = "amdgpu-profile-switcher.ron";

fn main() {
    let config_path = config_path();

    {
        let main_opt = MainOpt::parse();

        if main_opt.dump_procs {
            let procs = ProcProgEntry::get_all_proc_prog_entries();
            let procs: Vec<_> = procs.iter().map(|p| p.name.clone()).collect();
            println!("{procs:#?}");
            return;
        }

        if main_opt.check_config {
            let config = load_config(&config_path);
            println!("config_path: {config_path:?}");
            println!("{config:#?}");
            return;
        }
    }

    let config = load_config(&config_path);

    let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

    if pci_devs.is_empty() {
        panic!("No AMDGPU devices.");
    }

    let app_devices: Vec<_> = config.config_devices.iter().filter_map(|config_device| {
        let Some(pci) = pci_devs.iter().find(|&pci| &config_device.pci == pci) else {
            eprintln!("{} is not installed.", config_device.pci);
            return None;
        };
        let amdgpu_device = AmdgpuDevice::get_from_pci_bus(*pci)?;
        let config_device = config_device.clone();

        Some(AppDevice { amdgpu_device, config_device })
    }).collect();

    if app_devices.is_empty() {
        panic!("No available AMDGPU devices.");
    }

    env_logger::init();
    debug!("run loop");

    loop {
        let procs = ProcProgEntry::get_all_proc_prog_entries();

        for app in &app_devices {
            let mut apply_config_entry: Option<&ParsedConfigEntry> = None;

            for e in &app.config_device.entries {
                if procs.iter().any(|p| e.name == p.name) {
                    apply_config_entry = Some(e);
                }
            }

            if let Some(apply_config) = apply_config_entry {
                if let Some(perf_level) = apply_config.perf_level {
                    app.set_perf_level(perf_level);
                    debug!("Apply {perf_level:?} to {}", app.amdgpu_device.pci_bus);
                }
                if let Some(profile) = apply_config.profile {
                    app.set_power_profile(profile);
                    debug!("Apply {profile:?} to {}", app.amdgpu_device.pci_bus);
                }
            } else {
                app.reset_perf_level();
                app.reset_power_profile();
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn config_path() -> PathBuf {
    use std::env;
    use std::path::PathBuf;

    env::var("APS_CONFIG_PATH").ok().map(|s| PathBuf::from(s)).unwrap_or_else(|| {
        let config_home = env::var("XDG_CONFIG_HOME").unwrap_or("./".to_string());
        PathBuf::from(config_home).join(CONFIG_FILE)
    })
}

fn load_config(config_path: &Path) -> ParsedConfig {
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
