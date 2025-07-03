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

    pub fn set_default_perf_level(&self) {
        let perf_level = self.config_device.default_perf_level;
        let current_perf_level = DpmForcedLevel::get_from_sysfs(&self.amdgpu_device.sysfs_path)
            .expect("Error: Failed to get current dpm force performance level.");

        if current_perf_level != perf_level {
            fs::write(&self.amdgpu_device.dpm_perf_level_path, perf_level.to_arg())
                .unwrap_or_else(|e| panic!("IO Error: {e}"));
        }
    }

    pub fn set_power_profile(&self, profile: PowerProfile) {
        let profile = (profile as u32).to_string();
        fs::write(&self.amdgpu_device.power_profile_path, profile)
            .unwrap_or_else(|e| panic!("IO Error: {e}"));
    }

    pub fn set_default_power_profile(&self) {
        let profile = self.config_device.default_profile;
        let current_profile = PowerProfile::get_current_profile_from_sysfs(&self.amdgpu_device.sysfs_path)
            .expect("Error: Failed to get current power profile.");

        if current_profile != profile {
            let profile = (profile as u32).to_string();
            fs::write(&self.amdgpu_device.power_profile_path, profile)
                .unwrap_or_else(|e| panic!("IO Error: {e}"));
        }
    }

    pub fn set_default_power_cap(&self) {
        let power_cap_path = self.amdgpu_device.hwmon_path.join("power1_cap");
        let Some(target_power_cap_watt) = self.config_device.default_power_cap_watt
            else { return };
        let Some(current_power_cap_watt) = std::fs::read_to_string(power_cap_path)
            .ok()
            .and_then(|s| s.trim_end().parse::<u32>().ok())
            .and_then(|v| v.checked_div(1_000_000)) else { return };

        if target_power_cap_watt != current_power_cap_watt {
            let power_cap = (target_power_cap_watt * 1_000_000).to_string();
            fs::write(&self.amdgpu_device.hwmon_path.join("power1_cap"), power_cap)
                .unwrap_or_else(|e| panic!("IO Error: {e}"));
        }
    }

    pub fn name_list(&self) -> Vec<String> {
        self.config_device.names()
    }
}
