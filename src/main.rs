use std::sync::atomic::Ordering;

use libdrm_amdgpu_sys::AMDGPU;

use proc_prog_name::ProcProgEntry;
use log::debug;

mod config;
use config::{ConfigPerDevice, ParsedConfigEntry};

mod amdgpu_device;
use amdgpu_device::AmdgpuDevice;

mod args;
use args::{AppMode, MainOpt, SubCommand};

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
    let config_path = utils::config_path();

    if config_path.is_none() {
        eprintln!("Can't find the config file");
    }

    {
        let main_opt = MainOpt::parse();

        match main_opt.sub_command {
            SubCommand::AddEntry((pci, index, entry)) => {
                let config_path = config_path.unwrap();
                let mut config = utils::load_raw_config(&config_path);
                let pci = if let Some(pci) = pci {
                    pci
                } else if let Some(index) = index {
                    config.config_devices
                        .get(index)
                        .and_then(|device| device.pci.parse().ok())
                        .unwrap()
                } else {
                    panic!("Both `--pci` and `-i/--index` are empty.");
                };

                if let Some(config_device) = config.config_devices
                    .iter_mut()
                    .find(|device| pci == device.pci.parse().unwrap())
                {
                    config_device.entries.insert(0, entry);
                } else {
                    let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

                    if !pci_devs.iter().any(|pci_dev| pci_dev == &pci) {
                        pci_list!(pci_devs, pci);
                    }

                    let add_config_device = ConfigPerDevice {
                        pci: pci.to_string(),
                        _device_name: None,
                        default_power_cap_watt: None,
                        _power_cap_watt_range: None,
                        default_perf_level: None,
                        default_profile: None,
                        default_fan_target_temperature: None,
                        _fan_target_temperature_range: None,
                        default_fan_minimum_pwm: None,
                        _fan_minimum_pwm_range: None,
                        sclk_offset: None,
                        _sclk_offset_range: None,
                        vddgfx_offset: None,
                        _vddgfx_offset_range: None,
                        fan_zero_rpm: None,
                        entries: vec![entry],
                    };

                    config.config_devices.push(add_config_device);
                }

                utils::save_config_file(&config_path, &config).unwrap();

                return;
            },
            _ => {},
        }

        match main_opt.app_mode {
            AppMode::DumpProcs => {
                let procs = ProcProgEntry::get_all_proc_prog_entries();
                let procs: Vec<_> = procs.iter().map(|p| p.name.clone()).collect();
                println!("{procs:#?}");
                return;
            },
            AppMode::CheckConfig => {
                let config_path = config_path.unwrap();
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
            AppMode::DumpSupportedPowerProfile => {
                let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

                for pci in pci_devs {
                    let Some(amdgpu_device) = AmdgpuDevice::get_from_pci_bus(pci) else {
                        continue
                    };
                    let profiles = amdgpu_device.get_all_supported_power_profile();

                    println!(
                        "{} ({:#X}:{:#X}, {}): {:#?}",
                        amdgpu_device.device_name,
                        amdgpu_device.device_id,
                        amdgpu_device.revision_id,
                        amdgpu_device.pci_bus,
                        profiles,
                    );
                }

                return;
            },
            AppMode::DeviceList => {
                let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();

                for pci in pci_devs {
                    let Some(amdgpu_device) = AmdgpuDevice::get_from_pci_bus(pci) else {
                        continue
                    };

                    println!(
                        "{} ({:#X}:{:#X}, {})",
                        amdgpu_device.device_name,
                        amdgpu_device.device_id,
                        amdgpu_device.revision_id,
                        amdgpu_device.pci_bus,
                    );
                }

                return;
            },
            AppMode::Run => {},
        }
    }

    let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();
    let config_path = config_path.unwrap();

    if pci_devs.is_empty() {
        panic!("No AMDGPU devices.");
    }

    let mut app_devices: Vec<_> = {
        let config = utils::load_config(&config_path);
        config.config_devices.iter().filter_map(|config_device| {
            let Some(pci) = pci_devs.iter().find(|&pci| &config_device.pci == pci) else {
                pci_list!(pci_devs, config_device.pci);
            };
            let amdgpu_device = AmdgpuDevice::get_from_pci_bus(*pci)?;
            let config_device = config_device.clone();

            Some(AppDevice { amdgpu_device, config_device, cache_pid: None })
        }).collect()
    };

    if app_devices.is_empty() {
        panic!("No available AMDGPU devices.");
    }

    env_logger::init();
    debug!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    for app in &app_devices {
        'wait: loop {
            if !app.check_if_device_is_active() {
                debug!(
                    "wait until {} ({}) is active...",
                    app.amdgpu_device.device_name,
                    app.amdgpu_device.pci_bus,
                );
                std::thread::sleep(std::time::Duration::from_secs(1));
            } else {
                break 'wait;
            }
        }
        debug!("check permissions");
        if !app.amdgpu_device.check_permissions() {
            panic!("Error: PermissionDenied for sysfs");
        }

        let res: std::io::Result<Vec<_>> = [
            app.set_default_perf_level(),
            app.set_default_power_profile(),
            app.set_default_power_cap(),
            app.set_default_fan_target_temp(),
            app.set_default_fan_minimum_pwm(),
            app.set_fan_zero_rpm(),
            app.set_sclk_offset(),
            app.set_vddgfx_offset(),
        ].into_iter().collect();

        res.unwrap();
    }

    let modified = utils::watch_config_file(&config_path);

    debug!("run loop");

    let mut name_list: Vec<String> = app_devices.iter().flat_map(|app| app.name_list()).collect();
    let mut procs: Vec<ProcProgEntry> = Vec::with_capacity(name_list.len());

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
                    let changed = app.config_device.is_default_changed(config_device);
                    app.config_device.clone_from(config_device);

                    if changed {
                        debug!(
                            "{} ({}):re-aplly default config",
                            app.amdgpu_device.device_name,
                            app.amdgpu_device.pci_bus,
                        );

                        let _: std::io::Result<Vec<_>> = [
                            app.set_default_perf_level(),
                            app.set_default_power_profile(),
                            app.set_default_power_cap(),
                            app.set_default_fan_target_temp(),
                            app.set_default_fan_minimum_pwm(),
                            app.set_fan_zero_rpm(),
                            app.set_sclk_offset(),
                            app.set_vddgfx_offset(),
                        ].into_iter().collect();
                    }
                } else if let Some(pci) = pci_devs.iter().find(|&pci_dev| pci_dev == &config_device.pci) {
                    let new_app = AppDevice {
                        amdgpu_device: AmdgpuDevice::get_from_pci_bus(*pci).unwrap(),
                        config_device: config_device.clone(),
                        cache_pid: None,
                    };

                    app_devices.push(new_app);
                } else {
                    pci_list!(pci_devs, config_device.pci);
                }

                name_list.extend(config_device.names());
            }

            modified.store(false, Ordering::Release);
        }

        if !name_list.is_empty() {
            ProcProgEntry::update_entries_with_name_filter(&mut procs, &name_list);
        }

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

            if !app.check_if_device_is_active() {
                continue 'device;
            }

            if let Some(apply_config) = &apply_config_entry {
                debug!("target process: {}", apply_config.name);
                if let Some(perf_level) = apply_config.perf_level {
                    let _ = app.set_perf_level(perf_level);
                    debug!(
                        "Apply {perf_level:?} to {} ({})",
                        app.amdgpu_device.device_name,
                        app.amdgpu_device.pci_bus,
                    );
                }
                if let Some(profile) = apply_config.profile {
                    let _ = app.set_power_profile(profile);
                    debug!(
                        "Apply {profile:?} to {} ({})",
                        app.amdgpu_device.device_name,
                        app.amdgpu_device.pci_bus,
                    );
                }
                if let Some(power_cap_watt) = apply_config.power_cap_watt {
                    let _ = app.set_power_cap(power_cap_watt);
                    debug!(
                        "Apply {power_cap_watt}W cap. to {} ({})",
                        app.amdgpu_device.device_name,
                        app.amdgpu_device.pci_bus,
                    );
                }
                if let Some(target_temp) = apply_config.fan_target_temperature {
                    let _ = app.set_fan_target_temp(target_temp);
                    debug!(
                        "Apply fan_target_temperature {target_temp}C to {} ({})",
                        app.amdgpu_device.device_name,
                        app.amdgpu_device.pci_bus,
                    );
                }
                if let Some(minimum_pwm) = apply_config.fan_minimum_pwm {
                    let _ = app.set_fan_minimum_pwm(minimum_pwm);
                    debug!(
                        "Apply fan_minimum_pwm {minimum_pwm}% to {} ({})",
                        app.amdgpu_device.device_name,
                        app.amdgpu_device.pci_bus,
                    );
                }
                app.cache_pid = pid;
            } else if app.cache_pid.is_some() {
                debug!(
                    "set default perf_level ({:?}) and power_profile ({:?})",
                    app.config_device.default_perf_level,
                    app.config_device.default_profile,
                );
                if let Some(power_cap) = &app.config_device.default_power_cap_watt {
                    debug!("set default power cap. ({power_cap}W)");
                }
                let _ = app.set_default_perf_level();
                let _ = app.set_default_power_profile();
                let _ = app.set_default_power_cap();
                let _ = app.set_default_fan_target_temp();
                let _ = app.set_default_fan_minimum_pwm();
                app.cache_pid = None;
            }
        }

        procs.clear();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
