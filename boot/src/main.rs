// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use r_efi::efi;

// UEFI strings are NUL-terminated UTF-16; CRLF is required for line breaks.
const GREETING: &[u16] = &[
    72, 101, 108, 108, 111, 32, 77, 117, 115, 104, 97, 45, 79, 83, 33, 13, 10, 0,
];
const STAGE: &[u16] = &[
    85, 69, 70, 73, 32, 98, 111, 111, 116, 32, 115, 109, 111, 107, 101, 32,
    116, 101, 115, 116, 46, 32, 80, 114, 101, 115, 115, 32, 97, 110, 121, 32,
    107, 101, 121, 32, 116, 111, 32, 114, 101, 116, 117, 114, 110, 46, 13, 10, 0,
];

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Boot Services are not available through a global panic context.
    // Remain stopped rather than returning through an unknown stack frame.
    loop { core::hint::spin_loop(); }
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(
    _image: efi::Handle,
    system_table: *mut efi::SystemTable,
) -> efi::Status {
    if system_table.is_null() { return efi::Status::INVALID_PARAMETER; }
    // SAFETY: The firmware supplies a valid SystemTable for the lifetime of
    // this invocation. We have not called ExitBootServices. Protocol pointers
    // are checked before use; no references escape or are stored globally.
    unsafe {
        let table = &*system_table;
        if table.con_out.is_null() || table.con_in.is_null() || table.boot_services.is_null() {
            return efi::Status::UNSUPPORTED;
        }
        let output = table.con_out;
        for text in [GREETING, STAGE] {
            let status = ((*output).output_string)(output, text.as_ptr().cast_mut());
            if status.is_error() { return status; }
        }
        let input = table.con_in;
        // Clear any stale keystrokes, then wait for a fresh key event.
        let status = ((*input).reset)(input, false.into());
        if status.is_error() { return status; }
        let mut event = (*input).wait_for_key;
        if event.is_null() { return efi::Status::UNSUPPORTED; }
        let mut index = 0usize;
        let status = ((*table.boot_services).wait_for_event)(1, &mut event, &mut index);
        if status.is_error() { return status; }
        let mut key = core::mem::MaybeUninit::uninit();
        ((*input).read_key_stroke)(input, key.as_mut_ptr())
    }
}
