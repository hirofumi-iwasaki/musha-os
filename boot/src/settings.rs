// SPDX-License-Identifier: Apache-2.0
//! Firmware-only config access. No protocol pointer or file handle escapes this module.
use musha_platform::config::{
    Config, LogLevel, MAX_CONFIG_BYTES, fat32_boot_sector, force_devices, parse,
};
use r_efi::{
    efi,
    protocols::{block_io, file, simple_file_system, simple_text_input},
};
#[derive(Clone, Copy, Debug)]
pub enum ReadState {
    Loaded,
    MissingDirectory,
    MissingFile,
    Unavailable,
    NonFat32,
    Malformed,
    ReadError,
}
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub config: Config,
    pub state: ReadState,
    pub forced: bool,
}
use core::sync::atomic::{AtomicBool, Ordering};
static CLOSE_FAILED: AtomicBool = AtomicBool::new(false);
struct RawHash(core::cell::UnsafeCell<Option<[u8; 32]>>);
unsafe impl Sync for RawHash {} // Written once by BSP before ExitBootServices.
static RAW_HASH: RawHash = RawHash(core::cell::UnsafeCell::new(None));
/// CPU-owned digest only, retained after all CONFIG handles and buffers are gone.
/// Stage6 supplies the successful pairing + durable-record acknowledgement.
pub fn pairing_config_hash() -> Option<[u8; 32]> {
    unsafe { *RAW_HASH.0.get() }
}

struct Handle(*mut file::Protocol);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let status = ((*self.0).close)(self.0);
            if status.is_error() {
                CLOSE_FAILED.store(true, Ordering::Relaxed);
                crate::debug(b"MUSHA: CONFIG FILE CLOSE ERROR\n");
            }
        }
    }
}
unsafe fn open(parent: &Handle, name: &[u8]) -> Result<Handle, efi::Status> {
    let mut path = [0u16; 32];
    if name.len() >= path.len() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    for (to, &b) in path.iter_mut().zip(name) {
        *to = b as u16;
    }
    let mut ptr = core::ptr::null_mut();
    let status =
        unsafe { ((*parent.0).open)(parent.0, &mut ptr, path.as_mut_ptr(), file::MODE_READ, 0) };
    if status.is_error() {
        Err(status)
    } else if ptr.is_null() {
        Err(efi::Status::DEVICE_ERROR)
    } else {
        Ok(Handle(ptr))
    }
}
#[repr(align(4096))]
struct Sector([u8; 4096]);
unsafe fn load(
    bs: *mut efi::BootServices,
    device: efi::Handle,
    raw_hash: &mut Option<[u8; 32]>,
) -> (Config, ReadState) {
    let defaults = Config::default();
    let mut ptr = core::ptr::null_mut();
    let mut guid = block_io::PROTOCOL_GUID;
    if unsafe { ((*bs).handle_protocol)(device, &mut guid, &mut ptr) }.is_error() || ptr.is_null() {
        return (defaults, ReadState::Unavailable);
    }
    let block = ptr as *mut block_io::Protocol;
    let media = unsafe { (*block).media };
    if media.is_null() {
        return (defaults, ReadState::ReadError);
    }
    let media = unsafe { &*media };
    if !media.media_present
        || !matches!(media.block_size, 512 | 4096)
        || media.io_align > 4096
        || (media.io_align > 1 && !media.io_align.is_power_of_two())
    {
        return (defaults, ReadState::NonFat32);
    }
    let mut sector = Sector([0; 4096]);
    let status = unsafe {
        ((*block).read_blocks)(
            block,
            media.media_id,
            0,
            media.block_size as usize,
            sector.0.as_mut_ptr().cast(),
        )
    };
    if status.is_error() {
        return (defaults, ReadState::ReadError);
    }
    if !fat32_boot_sector(
        &sector.0,
        media.block_size,
        media.last_block.saturating_add(1),
    ) {
        return (defaults, ReadState::NonFat32);
    }
    ptr = core::ptr::null_mut();
    guid = simple_file_system::PROTOCOL_GUID;
    if unsafe { ((*bs).handle_protocol)(device, &mut guid, &mut ptr) }.is_error() || ptr.is_null() {
        return (defaults, ReadState::Unavailable);
    }
    let fs = ptr as *mut simple_file_system::Protocol;
    let mut root = core::ptr::null_mut();
    if unsafe { ((*fs).open_volume)(fs, &mut root) }.is_error() || root.is_null() {
        return (defaults, ReadState::ReadError);
    }
    let root = Handle(root);
    let directory = match unsafe { open(&root, b"SETTINGS") } {
        Ok(handle) => handle,
        Err(efi::Status::NOT_FOUND) => return (defaults, ReadState::MissingDirectory),
        Err(_) => return (defaults, ReadState::ReadError),
    };
    let file = match unsafe { open(&directory, b"CONFIG.TXT") } {
        Ok(handle) => handle,
        Err(efi::Status::NOT_FOUND) => return (defaults, ReadState::MissingFile),
        Err(_) => return (defaults, ReadState::ReadError),
    };
    let mut bytes = [0u8; MAX_CONFIG_BYTES + 1];
    let mut used = 0;
    for _ in 0..=MAX_CONFIG_BYTES {
        let mut count = bytes.len() - used;
        let status =
            unsafe { ((*file.0).read)(file.0, &mut count, bytes[used..].as_mut_ptr().cast()) };
        if status.is_error() || count > bytes.len() - used {
            return (defaults, ReadState::ReadError);
        }
        used += count;
        if used > MAX_CONFIG_BYTES {
            return (defaults, ReadState::Malformed);
        }
        if count == 0 {
            return match parse(&bytes[..used]) {
                Ok(config) => {
                    *raw_hash = Some(musha_platform::config_hash::sha256(&bytes[..used]));
                    (config, ReadState::Loaded)
                }
                Err(_) => (defaults, ReadState::Malformed),
            };
        }
    }
    (defaults, ReadState::ReadError)
}
/// Called before GOP selection. All load() handles close before key polling or return.
pub unsafe fn read(
    bs: *mut efi::BootServices,
    table: *mut efi::SystemTable,
    device: efi::Handle,
) -> Settings {
    CLOSE_FAILED.store(false, Ordering::Relaxed);
    let mut raw_hash = None;
    let (mut config, mut state) = unsafe { load(bs, device, &mut raw_hash) };
    if CLOSE_FAILED.load(Ordering::Relaxed) {
        config = Config::default();
        state = ReadState::ReadError;
        raw_hash = None;
    }
    crate::debug(b"MUSHA: CONFIG FILE CLEANUP COMPLETE\n");
    #[cfg(feature = "startup-config-probe")]
    {
        crate::debug(b"MUSHA: CONFIG KEY WINDOW\n");
        unsafe {
            ((*bs).stall)(2_000_000);
        }
    }
    let input = unsafe { (*table).con_in };
    let mut forced = false;
    if !input.is_null() {
        for _ in 0..32 {
            let mut key = simple_text_input::InputKey::default();
            if unsafe { ((*input).read_key_stroke)(input, &mut key) }.is_error() {
                break;
            }
            forced |= force_devices(key.scan_code, key.unicode_char);
        }
    }
    if forced {
        config.log = LogLevel::Devices;
    }
    unsafe {
        *RAW_HASH.0.get() = raw_hash;
    }
    Settings {
        config,
        state,
        forced,
    }
}
