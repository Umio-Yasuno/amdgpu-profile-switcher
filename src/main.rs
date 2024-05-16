use std::sync::atomic::Ordering;

use libdrm_amdgpu_sys::AMDGPU;

use proc_prog_name::ProcProgEntry;
use log::debug;

mod config;
use config::ParsedConfigEntry;

mod amdgpu_device;
use amdgpu_device::AmdgpuDevice;

mod args;
use args::MainOpt;

mod utils;

mod app;
use app::AppDevice;

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
            eprintln!("{pci_devs:#?}");
            return None;
        };
        let amdgpu_device = AmdgpuDevice::get_from_pci_bus(*pci)?;
        let config_device = config_device.clone();

        Some(AppDevice { amdgpu_device, config_device, cache_pid: None })
    }).collect();

    if app_devices.is_empty() {
        panic!("No available AMDGPU devices.");
    }

    let is_modified = utils::watch_config_file(&config_path);

    env_logger::init();
    debug!("run loop");

    let mut procs: Vec<ProcProgEntry> = Vec::with_capacity(128);
    let mut name_list: Vec<String> = app_devices.iter().map(|app| app.name_list()).flatten().collect();

    loop {
        if is_modified.load(Ordering::Acquire) {
            debug!("Reload config file");
            let config = utils::load_config(&config_path);

            name_list.clear();

            for app in app_devices.iter_mut() {
                app.update_config(&config.config_devices);
            }

            for config_device in &config.config_devices {
                name_list.extend(config_device.names());
            }

            is_modified.store(false, Ordering::Release);
        }

        ProcProgEntry::update_entries_with_name_filter(&mut procs, &name_list);

        'device: for app in app_devices.iter_mut() {
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
                app.reset_perf_level();
                app.reset_power_profile();
                app.cache_pid = None;
            }
        }

        procs.clear();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
