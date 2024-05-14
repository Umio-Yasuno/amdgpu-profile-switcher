use libdrm_amdgpu_sys::AMDGPU;
use AMDGPU::{DpmForcedLevel, PowerProfile};

use proc_prog_name::ProcProgEntry;
use log::debug;

pub mod config;
use config::{ParsedConfigPerDevice, ParsedConfigEntry};

mod amdgpu_device;
use amdgpu_device::AmdgpuDevice;

mod args;
use args::MainOpt;

mod utils;

struct AppDevice {
    pub amdgpu_device: AmdgpuDevice,
    pub config_device: ParsedConfigPerDevice,
    pub cache_config_entry: Option<ParsedConfigEntry>,
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

    fn update_config(&mut self, config_devices: &[ParsedConfigPerDevice]) {
        if let Some(config_device) = config_devices.iter().find(|config_dev| self.amdgpu_device.pci_bus == config_dev.pci) {
            self.config_device.clone_from(&config_device);
        }
    }
}

fn main() {
    let config_path = utils::config_path().expect("Config file is not found.");

    {
        let main_opt = MainOpt::parse();

        if main_opt.dump_procs {
            let procs = ProcProgEntry::get_all_proc_prog_entries();
            let procs: Vec<_> = procs.iter().map(|p| p.name.clone()).collect();
            println!("{procs:#?}");
            return;
        }

        if main_opt.check_config {
            let config = utils::load_config(&config_path);
            println!("config_path: {config_path:?}");
            println!("{config:#?}");
            return;
        }
    }

    let config = utils::load_config(&config_path);

    let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

    if pci_devs.is_empty() {
        panic!("No AMDGPU devices.");
    }

    let mut app_devices: Vec<_> = config.config_devices.iter().filter_map(|config_device| {
        let Some(pci) = pci_devs.iter().find(|&pci| &config_device.pci == pci) else {
            eprintln!("{} is not installed.", config_device.pci);
            return None;
        };
        let amdgpu_device = AmdgpuDevice::get_from_pci_bus(*pci)?;
        let config_device = config_device.clone();

        Some(AppDevice { amdgpu_device, config_device, cache_config_entry: None })
    }).collect();

    if app_devices.is_empty() {
        panic!("No available AMDGPU devices.");
    }

    let is_modified = utils::watch_config_file(&config_path);

    env_logger::init();
    debug!("run loop");

    let mut procs: Vec<ProcProgEntry> = Vec::with_capacity(128);

    use std::sync::atomic::Ordering;

    loop {
        if is_modified.load(Ordering::Acquire) {
            debug!("Reload config file");
            let config = utils::load_config(&config_path);

            for app in app_devices.iter_mut() {
                app.update_config(&config.config_devices);
            }

            is_modified.store(false, Ordering::Release);
        }

        ProcProgEntry::get_all_entries_with_buffer(&mut procs);

        'device: for app in app_devices.iter_mut() {
            let mut apply_config_entry: Option<ParsedConfigEntry> = None;

            'detect: for e in &app.config_device.entries {
                if procs.iter().any(|p| e.name == p.name) {
                    apply_config_entry = Some(e.clone());
                    break 'detect;
                }
            }

            if let [Some(detected), Some(cache_entry)] = [&apply_config_entry, &app.cache_config_entry] {
                if detected.name == cache_entry.name {
                    continue 'device;
                }
            }

            if let Some(apply_config) = &apply_config_entry {
                debug!("target process: {}", apply_config.name);
                if let Some(perf_level) = apply_config.perf_level {
                    app.set_perf_level(perf_level);
                    debug!("Apply {perf_level:?} to {}", app.amdgpu_device.pci_bus);
                }
                if let Some(profile) = apply_config.profile {
                    app.set_power_profile(profile);
                    debug!("Apply {profile:?} to {}", app.amdgpu_device.pci_bus);
                }
                app.cache_config_entry = apply_config_entry;
            } else if app.cache_config_entry.is_some() {
                debug!("reset perf_level and power_profile");
                app.reset_perf_level();
                app.reset_power_profile();
                app.cache_config_entry = None;
            }
        }

        procs.clear();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
