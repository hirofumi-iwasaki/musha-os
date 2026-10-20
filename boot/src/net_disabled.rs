// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! LAN-off facade for the cooperative runtime and diagnostic probes.
//! No network driver or DMA session is created in this build.

pub(crate) struct Session;

impl Session {
    pub(crate) fn start(
        _info: &crate::BootInfo,
        _time: &mut crate::acpi::Time,
    ) -> Result<Option<Self>, &'static str> {
        crate::bootlog::phase("lan", "disabled by build feature");
        Ok(None)
    }

    pub(crate) fn poll(&mut self, _now: u64) -> Result<(), &'static str> {
        Ok(())
    }

    pub(crate) fn udp_count(&self) -> u32 {
        0
    }

    pub(crate) fn started(&self) -> u64 {
        0
    }

    pub(crate) fn finish(&mut self, _fb: musha_framebuffer::Framebuffer) {}
}

pub(crate) fn failed(error: &str) {
    crate::diagnostics::net_failure(error);
}

#[cfg(any(feature = "live-file-probe", feature = "live-storage-probe"))]
pub(crate) fn probe_address_ready() -> bool {
    true
}
