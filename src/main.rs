use std::sync::atomic::Ordering;

use libdrm_amdgpu_sys::AMDGPU;

use proc_prog_name::ProcProgEntry;
use log::debug;

mod config;
use config::ParsedConfigEntry;

mod amdgpu_device;
use amdgpu_device::AmdgpuDevice;

mod args;
use args::{AppMode, MainOpt};

mod utils;

mod app;
use app::AppDevice;

macro_rules! pci_list {
    ($pci_devs:expr, $config_device_pci:expr) => {
        let pci_devs: Vec<_> = $pci_devs.iter().map(|pci| pci.to_string()).collect();
        eprintln!("{} is not installed or is not AMDGPU device.", $config_device_pci);
        eprintln!("AMDGPU list: {pci_devs:#?}");
        panic!();
    };
}

fn main() {
    let config_path = utils::config_path().expect("Config file is not found.");

    {
        let main_opt = MainOpt::parse();

        match main_opt.app_mode {
            AppMode::DumpProcs => {
                let procs = ProcProgEntry::get_all_proc_prog_entries();
                let procs: Vec<_> = procs.iter().map(|p| p.name.clone()).collect();
                println!("{procs:#?}");
                return;
            },
            AppMode::CheckConfig => {
                let config = utils::load_config(&config_path);
                println!("config_path: {config_path:?}");
                println!("{config:#?}");
                return;
            },
            AppMode::GenerateConfig => {
                let raw_config = utils::generate_config().unwrap();
                println!("{}{raw_config}", utils::COMMENT);
                return;
            },
            _ => {},
        }
    }

    let config = utils::load_config(&config_path);

    let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

    if pci_devs.is_empty() {
        panic!("No AMDGPU devices.");
    }

    let mut app_devices: Vec<_> = config.config_devices.iter().filter_map(|config_device| {
        let Some(pci) = pci_devs.iter().find(|&pci| &config_device.pci == pci) else {
            pci_list!(pci_devs, config_device.pci);
        };
        let amdgpu_device = AmdgpuDevice::get_from_pci_bus(*pci)?;
        let config_device = config_device.clone();

        Some(AppDevice { amdgpu_device, config_device, cache_pid: None })
    }).collect();

    if app_devices.is_empty() {
        panic!("No available AMDGPU devices.");
    }

    for app in &app_devices {
        debug!("set default power profile ({})", app.config_device.default_profile);
        app.set_default_perf_level();
        app.set_default_power_profile();
    }

    let modified = utils::watch_config_file(&config_path);

    env_logger::init();
    debug!("run loop");

    let mut procs: Vec<ProcProgEntry> = Vec::with_capacity(128);
    let mut name_list: Vec<String> = app_devices.iter().map(|app| app.name_list()).flatten().collect();

    loop {
        if modified.load(Ordering::Acquire) {
            debug!("Reload config file");
            let config = utils::load_config(&config_path);

            name_list.clear();

            for config_device in &config.config_devices {
                if let Some(ref mut app) = app_devices
                    .iter_mut()
                    .find(|app| app.amdgpu_device.pci_bus == config_device.pci)
                {
                    app.config_device.clone_from(config_device);
                } else {
                    pci_list!(pci_devs, config_device.pci);
                }

                name_list.extend(config_device.names());
            }

            modified.store(false, Ordering::Release);
        }

        ProcProgEntry::update_entries_with_name_filter(&mut procs, &name_list);

        'device: for app in app_devices.iter_mut() {
            if app.config_device.entries.is_empty() {
                continue 'device;
            }

            let mut apply_config_entry: Option<ParsedConfigEntry> = None;
            let mut pid: Option<i32> = None;

            'detect: for e in &app.config_device.entries {
                if let Some(proc) = procs.iter().find(|p| e.name == p.name) {
                    apply_config_entry = Some(e.clone());
                    pid = Some(proc.pid);
                    break 'detect;
                }
            }

            if app.cache_pid.is_some() && pid == app.cache_pid {
                continue 'device;
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
                app.cache_pid = pid;
            } else if app.cache_pid.is_some() {
                debug!("reset perf_level and power_profile");
                app.set_default_perf_level();
                app.set_default_power_profile();
                app.cache_pid = None;
            }
        }

        procs.clear();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
