use libdrm_amdgpu_sys::AMDGPU::{self, ContextHandle, DeviceHandle};
use libdrm_amdgpu_sys::PCI;

pub struct AmdgpuDevice {
    pub pci_bus: PCI::BUS_INFO,
    pub ctx: ContextHandle, // must drop before DeviceHandle
    pub amdgpu_dev: DeviceHandle,
}

impl AmdgpuDevice {
    pub fn get_from_pci_bus(pci_bus: PCI::BUS_INFO) -> Option<Self> {
        let device_path = pci_bus.get_drm_render_path().ok()?;
        let (amdgpu_dev, _major, _minor) = {
            use std::fs::File;
            use std::os::fd::IntoRawFd;

            let fd = File::open(device_path).ok()?;

            DeviceHandle::init(fd.into_raw_fd()).ok()?
        };

        let ctx = amdgpu_dev.create_context().ok()?;

        Some(Self {
            pci_bus,
            ctx,
            amdgpu_dev,
        })
    }
}
