// SPDX-License-Identifier: Apache-2.0
#![no_std]
#![no_main]
mod common;
#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn musha_main(info: *const musha::MushaStartInfoV1) -> i32 {
    if info.is_null() { return 1; }
    let start = unsafe { info.read() };
    if musha::validate_start(&start).is_err() || start.abi_minor != 2 { return 2; }
    match run() { Ok(()) => 0, Err(_) => 3 }
}
fn run() -> Result<(), musha::Error> {
    use musha::guest::*;
    if query()? != 3 { return Err(musha::Error(-3)); }
    let start = time()?;
    if wait(0, false)? != 0 || wait(20, false)? != 0 || time()? < start+20 {
        return Err(musha::Error(-3));
    }
    log("R2 RUST TIME WAIT OK\n")?;
    while input().is_ok() {}
    log("R2 INPUT READY\n")?;
    let mut q_down = false;
    loop {
        if wait(5000, true)? != 1 { return Err(musha::Error(-7)); }
        loop {
            match input() {
                Err(musha::Error(-6)) => break,
                Err(error) => return Err(error),
                Ok(event) => {
                    if event.kind == 1 {
                        log(if event.flags == 1 { "R2 KEY DOWN\n" } else { "R2 KEY UP\n" })?;
                        if event.code == 0x14 {
                            if event.flags == 1 { q_down = true; }
                            else if q_down { log("R2 Q EXIT\n")?; return Ok(()); }
                        }
                    } else if event.kind == 2 {
                        log("R2 MOUSE\n")?;
                        common::number("DX ", event.value0 as i64 as u64).map_err(musha::Error)?;
                        common::number("DY ", event.value1 as i64 as u64).map_err(musha::Error)?;
                        common::number("WHEEL ", event.value2 as i64 as u64).map_err(musha::Error)?;
                        common::number("BUTTONS ", event.buttons as u64).map_err(musha::Error)?;
                    }
                }
            }
        }
    }
}
