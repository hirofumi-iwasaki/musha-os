// SPDX-License-Identifier: Apache-2.0
//! C2: immutable embedded ELF, permanent owned backing, one-shot execution.
use super::{abi, failed};
use musha_memory::{PAGE, elf::Plan, user::offset};
#[cfg(feature = "ring3-elf-probe")]
const FILE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/user.elf"));
#[derive(Clone, Copy)]
struct Mapping {
    va: usize,
    pa: usize,
    bytes: usize,
    executable_bytes: usize,
}
const EMPTY: Mapping = Mapping {
    va: 0,
    pa: 0,
    bytes: 0,
    executable_bytes: 0,
};
struct Loaded {
    maps: [Mapping; 35],
    count: usize,
    stack: usize,
    stack_bytes: usize,
    permissions: u64,
}
static mut LOADED: Loaded = Loaded {
    maps: [EMPTY; 35],
    count: 0,
    stack: 0,
    stack_bytes: 0,
    permissions: 0,
};
pub unsafe fn load(arena: &mut [u8]) -> usize {
    #[cfg(feature = "ring3-elf-probe")]
    let file = FILE;
    #[cfg(external_app)]
    let file = crate::external::image();
    let plan = Plan::parse(file).unwrap_or_else(|_| failed(b"ELF_FORMAT"));
    let needed = plan.backing_bytes().unwrap_or_else(|_| failed(b"ELF_SIZE"));
    let start = 20 * PAGE;
    if needed
        > arena
            .len()
            .checked_sub(start)
            .unwrap_or_else(|| failed(b"ELF_MEMORY"))
    {
        failed(b"ELF_MEMORY");
    }
    let stack = abi::STACK_TOP as usize - plan.stack_bytes;
    // Validate *all* virtual pages, including the guard, before installing any.
    for s in &plan.segments[..plan.count] {
        if unsafe { crate::memory::probe_vacant(s.page_base(), s.page_bytes()) }.is_err() {
            failed(b"ELF_COLLISION");
        }
    }
    for (va, bytes) in [
        (abi::ARENA_BASE as usize, plan.arena_bytes),
        (abi::START_BASE as usize, PAGE),
        (stack - PAGE, plan.stack_bytes + PAGE),
    ] {
        if unsafe { crate::memory::probe_vacant(va, bytes) }.is_err() {
            failed(b"ELF_COLLISION");
        }
    }
    let pa = arena.as_mut_ptr() as usize;
    let mut cursor = start;
    let mut state = Loaded {
        maps: [EMPTY; 35],
        count: 0,
        stack,
        stack_bytes: plan.stack_bytes,
        permissions: plan.permissions,
    };
    for i in 0..plan.count {
        let s = plan.segments[i];
        let bytes = s.page_bytes();
        plan.load_segment(i, file, &mut arena[cursor..cursor + bytes])
            .unwrap_or_else(|_| failed(b"ELF_COPY"));
        unsafe {
            map(
                s.page_base(),
                pa + cursor,
                bytes,
                s.flags == 6,
                s.flags == 5,
            );
        }
        state.maps[state.count] = Mapping {
            va: s.page_base(),
            pa: pa + cursor,
            bytes,
            executable_bytes: 0,
        };
        // Return RIP is restricted to the file-backed portion, not BSS/padding.
        if s.flags == 5 {
            state.maps[state.count].va = s.address;
            state.maps[state.count].pa += s.address - s.page_base();
            state.maps[state.count].bytes = s.memory_bytes;
            state.maps[state.count].executable_bytes = s.file_bytes;
        }
        state.count += 1;
        cursor += bytes;
    }
    for (va, bytes, write) in [
        (abi::ARENA_BASE as usize, plan.arena_bytes, true),
        (stack, plan.stack_bytes, true),
        (abi::START_BASE as usize, PAGE, false),
    ] {
        arena[cursor..cursor + bytes].fill(0);
        if va == abi::START_BASE as usize {
            let info = abi::MushaStartInfoV1 {
                struct_size: abi::START_SIZE as u32,
                abi_major: abi::ABI_MAJOR as u16,
                abi_minor: abi::ABI_MINOR as u16,
                arena_base: abi::ARENA_BASE,
                arena_bytes: plan.arena_bytes as u64,
                stack_base: stack as u64,
                stack_bytes: plan.stack_bytes as u64,
                ..Default::default()
            };
            unsafe {
                (pa.wrapping_add(cursor) as *mut abi::MushaStartInfoV1).write(info);
            }
        }
        unsafe {
            map(va, pa + cursor, bytes, write, false);
        }
        state.maps[state.count] = Mapping {
            va,
            pa: pa + cursor,
            bytes,
            executable_bytes: 0,
        };
        state.count += 1;
        cursor += bytes;
    }
    unsafe {
        core::ptr::addr_of_mut!(LOADED).write(state);
    }
    crate::debug(b"MUSHA: RING3_ELF_LOADED BSS_START_READY\n");
    plan.entry
}
unsafe fn map(va: usize, pa: usize, bytes: usize, write: bool, execute: bool) {
    for off in (0..bytes).step_by(PAGE) {
        #[cfg(feature = "ring3-lifecycle-map-failure")]
        if off == PAGE {
            failed(b"INJECTED_MAP_FAILURE");
        }
        if unsafe { crate::memory::probe_user_page(va + off, pa + off, write, execute) }.is_err() {
            failed(b"ELF_MAP");
        }
    }
}
pub fn valid_return(rip: u64, rsp: u64) -> bool {
    let s = unsafe { &*core::ptr::addr_of!(LOADED) };
    s.maps[..s.count].iter().any(|m| {
        m.executable_bytes > 0
            && musha_memory::user::valid_return(
                rip,
                rsp,
                m.va as u64,
                m.executable_bytes as u64,
                s.stack as u64,
                s.stack_bytes as u64,
            )
    })
}
pub fn log(pointer: u64, length: u64) -> (i32, u64) {
    let s = unsafe { &*core::ptr::addr_of!(LOADED) };
    if s.permissions & abi::PERMISSION_LOG == 0 {
        return (abi::ERR_DENIED, 0);
    }
    if length > abi::MAX_LOG_BYTES {
        return (abi::ERR_LIMIT, 0);
    }
    if pointer.checked_add(length).is_none() {
        return (abi::ERR_INVALID, 0);
    }
    let mut bytes = [0u8; 4096];
    // Bounded copy via permanent supervisor aliases. No user virtual dereference,
    // paging/reclamation, concurrent thread, or partially emitted failed copy.
    for i in 0..length as usize {
        let address = s.maps[..s.count].iter().find_map(|m| {
            offset(pointer + i as u64, 1, m.va as u64, m.bytes as u64).map(|n| m.pa + n)
        });
        let Some(address) = address else {
            return (abi::ERR_INVALID, 0);
        };
        bytes[i] = unsafe { (address as *const u8).read() };
    }
    if core::str::from_utf8(&bytes[..length as usize]).is_err() {
        return (abi::ERR_INVALID, 0);
    }
    #[cfg(external_app)]
    {
        crate::appconsole::append(&bytes[..length as usize]);
        crate::bootstate::frame();
    }
    crate::debug(&bytes[..length as usize]);
    (0, length)
}

#[cfg(feature = "ring3-lifecycle-probe")]
pub unsafe fn clear() {
    unsafe {
        core::ptr::addr_of_mut!(LOADED).write(Loaded {
            maps: [EMPTY; 35],
            count: 0,
            stack: 0,
            stack_bytes: 0,
            permissions: 0,
        });
    }
}
