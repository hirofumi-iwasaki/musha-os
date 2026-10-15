// SPDX-License-Identifier: Apache-2.0
#[allow(dead_code)]
pub fn number(label: &str, value: u64) -> Result<(), i32> {
    musha::guest::log(label).map_err(|_| 4)?;
    let mut bytes = *b"0x0000000000000000\n";
    for n in 0..16 {
        bytes[17 - n] = b"0123456789ABCDEF"[((value >> (n * 4)) & 15) as usize];
    }
    let text = core::str::from_utf8(&bytes).map_err(|_| 4)?;
    musha::guest::log(text).map_err(|_| 4)?;
    Ok(())
}
#[unsafe(no_mangle)]
pub extern "sysv64" fn musha_exit(status: i32) -> ! {
    musha::guest::exit(status)
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    musha::guest::exit(127)
}
