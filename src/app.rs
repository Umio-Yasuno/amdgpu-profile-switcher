use std::fs;

use libdrm_amdgpu_sys::AMDGPU;
use AMDGPU::{DpmForcedLevel, PowerProfile};

use crate::config::ParsedConfigPerDevice;
use crate::amdgpu_device::AmdgpuDevice;

pub struct AppDevice {
    pub amdgpu_device: AmdgpuDevice,
    pub config_device: ParsedConfigPerDevice,
    pub cache_pid: Option<i32>,
}

impl AppDevice {
    pub fn set_perf_level(&self, perf_level: DpmForcedLevel) {
        let perf_level = perf_level.to_arg();
        fs::write(&self.amdgpu_device.dpm_perf_level_path, perf_level)
            .unwrap_or_else(|e| panic!("IO Error: {e}"));
    }

    pub fn reset_perf_level(&self) {
        let current = DpmForcedLevel::get_from_sysfs(&self.amdgpu_device.sysfs_path)
            .expect("Error: Failed to get current dpm force performance level.");
        match current {
            DpmForcedLevel::Auto |
            DpmForcedLevel::Manual => {},
            _ => {
                fs::write(&self.amdgpu_device.dpm_perf_level_path, DpmForcedLevel::Auto.to_arg())
                    .unwrap_or_else(|e| panic!("IO Error: {e}"));
            },
        }
    }

    pub fn set_power_profile(&self, profile: PowerProfile) {
        let profile = (profile as u32).to_string();
        fs::write(&self.amdgpu_device.power_profile_path, profile)
            .unwrap_or_else(|e| panic!("IO Error: {e}"));
    }

    pub fn reset_power_profile(&self) {
        let current_profile = PowerProfile::get_current_profile_from_sysfs(&self.amdgpu_device.sysfs_path)
            .expect("Error: Failed to get current power profile.");
        if current_profile != PowerProfile::BOOTUP_DEFAULT {
            let profile = (PowerProfile::BOOTUP_DEFAULT as u32).to_string();
            fs::write(&self.amdgpu_device.power_profile_path, profile)
                .unwrap_or_else(|e| panic!("IO Error: {e}"));
        }
    }

    pub fn name_list(&self) -> Vec<String> {
        self.config_device.names()
    }
}
