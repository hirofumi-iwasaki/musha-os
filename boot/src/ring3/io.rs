// SPDX-License-Identifier: Apache-2.0
//! BSP/IF=0 one-shot input loan. Host state stays on OS stack until terminal stop.
use super::{abi, elf};
use core::cell::UnsafeCell;
use musha_platform::user_io::{Event, KEY, Queue};
type Poll = unsafe fn(*mut ()) -> Result<(), &'static str>;
struct State {
    queue: Option<Queue>,
    context: *mut (),
    poll: Option<Poll>,
    busy: bool,
    disconnected: bool,
    modifiers: u16,
    stop_info: *const crate::BootInfo,
}
struct Shared(UnsafeCell<State>);
// Only BSP with interrupts disabled; never retain a reference across a callback.
unsafe impl Sync for Shared {}
static STATE: Shared = Shared(UnsafeCell::new(State {
    queue: None,
    context: core::ptr::null_mut(),
    poll: None,
    busy: false,
    disconnected: false,
    modifiers: 0,
    stop_info: core::ptr::null(),
}));
/// # Safety
/// callback, all borrowed Hosts/rings and info must remain live until stop().
/// Caller must enter the non-returning Ring3 path immediately, without moving F.
pub unsafe fn attach<F: FnMut() -> Result<(), &'static str>>(
    callback: &mut F,
    info: &crate::BootInfo,
) {
    unsafe fn invoke<F: FnMut() -> Result<(), &'static str>>(
        p: *mut (),
    ) -> Result<(), &'static str> {
        unsafe { (&mut *(p as *mut F))() }
    }
    unsafe {
        let s = &mut *STATE.0.get();
        s.queue = Some(Queue::default());
        s.context = callback as *mut F as *mut ();
        s.poll = Some(invoke::<F>);
        s.stop_info = info;
        s.disconnected = false;
        s.modifiers = 0;
    }
}
pub fn time() -> (i32, u64) {
    crate::bootstate::epoch_now().map_or((abi::ERR_IO, 0), |ms| (0, ms))
}
pub fn poll() -> Result<(), i32> {
    let call = unsafe {
        let s = &mut *STATE.0.get();
        if s.disconnected {
            return Err(abi::ERR_DISCONNECTED);
        }
        if s.busy {
            return Err(abi::ERR_IO);
        }
        match s.poll {
            Some(f) => {
                s.busy = true;
                Some((f, s.context))
            }
            None => None,
        }
    };
    if let Some((f, context)) = call {
        let result = unsafe { f(context) };
        unsafe {
            (*STATE.0.get()).busy = false;
        }
        if result.is_err() {
            shutdown();
            unsafe {
                (*STATE.0.get()).disconnected = true;
            }
            return Err(abi::ERR_DISCONNECTED);
        }
    }
    Ok(())
}
pub fn keys(events: &[(u8, bool)]) {
    for &(code, down) in events {
        let now = crate::bootstate::epoch_now().unwrap_or(0);
        unsafe {
            let s = &mut *STATE.0.get();
            if (0xe0..=0xe7).contains(&code) {
                let mask = 1 << (code - 0xe0);
                if down {
                    s.modifiers |= mask;
                } else {
                    s.modifiers &= !mask;
                }
            }
            if let Some(q) = s.queue.as_mut() {
                q.push(Event {
                    kind: KEY,
                    flags: down as u32,
                    timestamp_ms: now,
                    device: 1,
                    code: code as u16,
                    modifiers: s.modifiers,
                    ..Event::default()
                });
            }
        }
    }
}
pub fn next() -> Option<Event> {
    let now = crate::bootstate::epoch_now().unwrap_or(0);
    unsafe { (*STATE.0.get()).queue.as_mut()?.pop(now) }
}
pub fn wait(timeout: u64, flags: u64) -> (i32, u64) {
    if !elf::permitted(abi::PERMISSION_WAIT)
        || flags & abi::WAIT_INPUT != 0 && !elf::permitted(abi::PERMISSION_INPUT)
    {
        return (abi::ERR_DENIED, 0);
    }
    if flags & !abi::WAIT_INPUT != 0 {
        return (abi::ERR_INVALID, 0);
    }
    if timeout > abi::MAX_WAIT_MS {
        return (abi::ERR_LIMIT, 0);
    }
    let Some(start) = crate::bootstate::epoch_now() else {
        return (abi::ERR_IO, 0);
    };
    let Some(end) = start.checked_add(timeout) else {
        return (abi::ERR_LIMIT, 0);
    };
    // Finite watchdog also covers a stuck timer; no IRQ/preemption is assumed.
    let mut last = start;
    let mut stalled = 0u32;
    loop {
        if let Err(error) = poll() {
            return (error, 0);
        }
        if flags & abi::WAIT_INPUT != 0
            && unsafe { (*STATE.0.get()).queue.as_ref().is_some_and(Queue::pending) }
        {
            return (0, abi::WAKE_INPUT);
        }
        let Some(now) = crate::bootstate::epoch_now() else {
            return (abi::ERR_IO, 0);
        };
        if now >= end {
            return (0, abi::WAKE_TIMEOUT);
        }
        if now == last {
            stalled += 1;
        } else {
            stalled = 0;
            last = now;
        }
        if stalled >= 1_000_000 {
            return (abi::ERR_IO, 0);
        }
        core::hint::spin_loop();
    }
}
pub fn shutdown() {
    let info = unsafe {
        let s = &mut *STATE.0.get();
        let info = s.stop_info;
        s.stop_info = core::ptr::null();
        s.poll = None;
        s.context = core::ptr::null_mut();
        info
    };
    if !info.is_null() {
        // Halting never reclaims the stack/arena/DMA pool, even on failure.
        let stopped = unsafe { crate::xhci::quiesce_app(&*info) };
        crate::debug(if stopped {
            b"MUSHA: APP_INPUT_STOPPED DMA_DISABLED\n"
        } else {
            b"MUSHA: APP_INPUT_STOP_UNCONFIRMED QUARANTINED\n"
        });
    }
}

pub fn pointer(values: [i32; 3], buttons: u32) {
    let now = crate::bootstate::epoch_now().unwrap_or(0);
    unsafe {
        if let Some(q) = (*STATE.0.get()).queue.as_mut() {
            q.push(Event {
                kind: musha_platform::user_io::MOUSE,
                timestamp_ms: now,
                device: 2,
                values,
                buttons,
                ..Event::default()
            });
        }
    }
}
