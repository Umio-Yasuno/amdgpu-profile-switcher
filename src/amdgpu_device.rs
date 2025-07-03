use std::path::PathBuf;
use std::fs;

use libdrm_amdgpu_sys::PCI;
use libdrm_amdgpu_sys::AMDGPU::{self, PowerProfile};

pub struct AmdgpuDevice {
    pub pci_bus: PCI::BUS_INFO,
    pub sysfs_path: PathBuf,
    pub hwmon_path: PathBuf,
    pub device_id: u32,
    pub revision_id: u32,
    pub device_name: String,
    pub power_profile_path: PathBuf,
    pub dpm_perf_level_path: PathBuf,
    pub default_power_cap_watt: Option<u32>,
}

impl AmdgpuDevice {
    pub fn get_from_pci_bus(pci_bus: PCI::BUS_INFO) -> Option<Self> {
        let sysfs_path = pci_bus.get_sysfs_path();
        let power_profile_path = sysfs_path.join("pp_power_profile_mode");
        let dpm_perf_level_path = sysfs_path.join("power_dpm_force_performance_level");

        if !power_profile_path.exists() || !dpm_perf_level_path.exists() {
            return None;
        }

        let [device_id, revision_id] = {
            let [did, rid] = ["device", "revision"]
                .map(|s| std::fs::read_to_string(sysfs_path.join(s)).ok());

            [did?, rid?]
                .map(|s|
                    u32::from_str_radix(s.trim_start_matches("0x").trim_end(), 16).unwrap()
                )
        };
        let device_name = AMDGPU::find_device_name(device_id, revision_id)
            .unwrap_or(AMDGPU::DEFAULT_DEVICE_NAME.to_string());
        let hwmon_path = pci_bus.get_hwmon_path()?;
        let power_cap_path = hwmon_path.join("power1_cap_default");
        let default_power_cap_watt = std::fs::read_to_string(power_cap_path)
            .ok()
            .and_then(|s| s.trim_end().parse::<u32>().ok())
            .and_then(|v| v.checked_div(1_000_000));

        Some(Self {
            pci_bus,
            sysfs_path,
            hwmon_path,
            device_id,
            revision_id,
            device_name,
            power_profile_path,
            dpm_perf_level_path,
            default_power_cap_watt,
        })
    }

    pub fn check_permissions(&self) -> bool {
        [&self.power_profile_path, &self.dpm_perf_level_path]
            .iter()
            .any(|path| {
                fs::OpenOptions::new().read(true).write(true).open(path).is_ok()
            })
    }

    pub fn get_all_supported_power_profile(&self) -> Vec<PowerProfile> {
        PowerProfile::get_all_supported_profiles_from_sysfs(&self.sysfs_path)
    }
}
