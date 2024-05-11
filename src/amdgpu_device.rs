use std::path::PathBuf;

use libdrm_amdgpu_sys::PCI;

pub struct AmdgpuDevice {
    pub pci_bus: PCI::BUS_INFO,
    pub sysfs_path: PathBuf,
    pub power_profile_path: PathBuf,
}

impl AmdgpuDevice {
    pub fn get_from_pci_bus(pci_bus: PCI::BUS_INFO) -> Option<Self> {
        let sysfs_path = pci_bus.get_sysfs_path();
        let power_profile_path = sysfs_path.join("pp_power_profile_mode");
        let dpm_perf_level_path = sysfs_path.join("power_dpm_force_performance_level");

        if !power_profile_path.exists() || !dpm_perf_level_path.exists() {
            return None;
        }

        Some(Self {
            pci_bus,
            sysfs_path,
            power_profile_path,
        })
    }
}
