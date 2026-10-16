// SPDX-License-Identifier: Apache-2.0
//! Application output uses the single kernel console; syscall limits are unchanged.
use musha_framebuffer::Framebuffer;
pub fn prepare(_fb: Framebuffer) {}
pub fn start(_fb: Framebuffer, text: &[u8]) {
    crate::bootlog::viewer(false);
    crate::bootlog::app(text);
}
pub fn append(bytes: &[u8]) {
    crate::bootlog::app(bytes);
}
pub fn message(args: core::fmt::Arguments<'_>) {
    crate::bootlog::event(musha_platform::boot_log::Level::Application, "", args);
}
pub fn rejected(reason: &[u8]) {
    message(format_args!(
        "app: rejected: {}",
        core::str::from_utf8(reason).unwrap_or("invalid reason")
    ));
    crate::bootstate::update(|r, _| r.finish(false));
}
pub unsafe fn fault(vector: u64, error: u64, rip: u64, rsp: u64) {
    #[cfg(external_app)]
    let name = crate::external::application_name();
    #[cfg(external_app)]
    let name = name.as_str();
    #[cfg(not(external_app))]
    let name = "application";
    if musha_platform::exceptions::hardware_error(vector) {
        message(format_args!(
            "app: {} faulted, vector={} error=0x{:x} rip=0x{:x} rsp=0x{:x}",
            name, vector, error, rip, rsp
        ));
    } else {
        message(format_args!(
            "app: {} faulted, vector={} error=n/a rip=0x{:x} rsp=0x{:x}",
            name, vector, rip, rsp
        ));
    }
    crate::bootstate::update(|r, _| r.finish(false));
}
