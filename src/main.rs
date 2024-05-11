use std::fs::File;

use libdrm_amdgpu_sys::AMDGPU;

use proc_prog_name::ProcProgEntry;
use ron::de;

mod config;
use config::Config;

mod amdgpu_device;
use amdgpu_device::AmdgpuDevice;

struct AppDevice {
    pub amdgpu_device: AmdgpuDevice,
    pub config_device: config::ParsedConfigPerDevice,
}

fn main() {
    let f = File::open("./proc_watch.ron").unwrap();
    let config: Config = match de::from_reader(f) {
        Ok(v) => v,
        Err(e) => {
            println!("{e:?}");
            panic!();
        },
    };
    let config = config.parse();

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

    'run: loop {
        let procs = ProcProgEntry::get_all_proc_prog_entries();

        for app in &app_devices {
            let mut apply_config_entry: Option<&config::ParsedConfigEntry> = None;

            for e in &app.config_device.entries {
                if procs.iter().find(|p| e.name == p.name).is_some() {
                    apply_config_entry = Some(e);
                }
                
            }
            println!("{:?} {apply_config_entry:?}", app.amdgpu_device.pci_bus);

            if let Some(entry) = apply_config_entry {
                if let Err(err) = app.amdgpu_device.ctx.set_stable_pstate(entry.pstate) {
                    println!("    Error: {err}");
                }
            } else {
                if let Err(err) = app.amdgpu_device.ctx.set_stable_pstate(AMDGPU::StablePstateFlag::NONE) {
                    println!("    Error: {err}");
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
