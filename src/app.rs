use std::{fs, io};

use libdrm_amdgpu_sys::AMDGPU;
use AMDGPU::{DpmForcedLevel, PowerProfile};

use crate::config::ParsedConfigPerDevice;
use crate::amdgpu_device::AmdgpuDevice;

pub struct AppDevice {
    pub amdgpu_device: AmdgpuDevice,
    pub config_device: ParsedConfigPerDevice,
    pub cache_pid: Option<i32>,
}

const IO_ERROR_POWER_CAP: &str = "Can't get the power cap";

impl AppDevice {
    pub fn set_perf_level(&self, perf_level: DpmForcedLevel) -> io::Result<()> {
        let perf_level = perf_level.to_arg();
        fs::write(&self.amdgpu_device.dpm_perf_level_path, perf_level)
    }

    pub fn set_default_perf_level(&self) -> io::Result<()> {
        let perf_level = self.config_device.default_perf_level;
        let current_perf_level = DpmForcedLevel::get_from_sysfs(&self.amdgpu_device.sysfs_path)?;

        if current_perf_level != perf_level {
            fs::write(&self.amdgpu_device.dpm_perf_level_path, perf_level.to_arg())
        } else {
            Ok(())
        }
    }

    pub fn set_power_profile(&self, profile: PowerProfile) -> io::Result<()> {
        let profile = (profile as u32).to_string();
        fs::write(&self.amdgpu_device.power_profile_path, profile)
    }

    pub fn set_default_power_profile(&self) -> io::Result<()> {
        let profile = self.config_device.default_profile;
        let Some(current_profile) = PowerProfile::get_current_profile_from_sysfs(&self.amdgpu_device.sysfs_path)
            else { return Err(io::Error::last_os_error()) };

        if current_profile != profile {
            let profile = (profile as u32).to_string();
            fs::write(&self.amdgpu_device.power_profile_path, profile)
        } else {
            Ok(())
        }
    }

    pub fn set_power_cap(&self, power_cap_watt: u32) -> io::Result<()> {
        let power_cap_path = self.amdgpu_device.hwmon_path.join("power1_cap");
        let Some(current_power_cap_watt) = std::fs::read_to_string(power_cap_path)
            .ok()
            .and_then(|s| s.trim_end().parse::<u32>().ok())
            .and_then(|v| v.checked_div(1_000_000))
            else { return Err(io::Error::other(IO_ERROR_POWER_CAP)) };

        if power_cap_watt != current_power_cap_watt {
            let power_cap = (power_cap_watt * 1_000_000).to_string();
            fs::write(self.amdgpu_device.hwmon_path.join("power1_cap"), power_cap)
        } else {
            Ok(())
        }
    }

    pub fn set_default_power_cap(&self) -> io::Result<()> {
        let power_cap_path = self.amdgpu_device.hwmon_path.join("power1_cap");
        let Some(target_power_cap_watt) = self.config_device.default_power_cap_watt
            else { return Err(io::Error::other(IO_ERROR_POWER_CAP)) };
        let Some(current_power_cap_watt) = std::fs::read_to_string(power_cap_path)
            .ok()
            .and_then(|s| s.trim_end().parse::<u32>().ok())
            .and_then(|v| v.checked_div(1_000_000))
            else { return Err(io::Error::other(IO_ERROR_POWER_CAP)) };

        if target_power_cap_watt != current_power_cap_watt {
            let power_cap = (target_power_cap_watt * 1_000_000).to_string();
            fs::write(self.amdgpu_device.hwmon_path.join("power1_cap"), power_cap)
        } else {
            Ok(())
        }
    }

    pub fn set_fan_target_temp(&self, target_temp: u32) -> io::Result<()> {
        let fan_target_temp_path = self
            .amdgpu_device
            .sysfs_path
            .join("gpu_od/fan_ctrl/fan_target_temperature");
        fs::write(&fan_target_temp_path, target_temp.to_string())?;
        fs::write(&fan_target_temp_path, "c")
    }

    pub fn set_default_fan_target_temp(&self) -> io::Result<()> {
        let fan_target_temp_path = self
            .amdgpu_device
            .sysfs_path
            .join("gpu_od/fan_ctrl/fan_target_temperature");
        let Some(target_temp) = self.config_device.default_fan_target_temperature
            else { return Err(io::Error::other("fan_target_temperature is None")) };
        fs::write(&fan_target_temp_path, target_temp.to_string())?;
        fs::write(&fan_target_temp_path, "c")
    }

    pub fn name_list(&self) -> Vec<String> {
        self.config_device.names()
    }

    pub fn check_if_device_is_active(&self) -> bool {
        let path = self.amdgpu_device.sysfs_path.join("power/runtime_status");
        let Ok(s) = std::fs::read_to_string(path) else { return false };

        s.starts_with("active")
    }
}
