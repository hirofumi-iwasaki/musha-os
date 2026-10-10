// SPDX-License-Identifier: Apache-2.0
//! Conservative policy, independent of machine model or vendor ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub id: u32,
    pub command: u16,
    pub bar0: u32,
    pub bar1: u32,
    pub pm: u32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Unchanged,
    Restore,
    Reject(&'static str),
}
pub fn decide(
    early: State,
    pre: State,
    post: State,
    now: State,
    base: usize,
    bytes: usize,
    owner: bool,
) -> Decision {
    if now.command & 2 != 0 {
        return Decision::Unchanged;
    }
    let reject = |s| Decision::Reject(s);
    if !owner {
        return reject("NOT BOOT OWNER");
    }
    if early != pre {
        return reject("PRE STATE CHANGED");
    }
    if pre.id == 0 || pre.id as u16 == 0xffff || post.id != pre.id || now.id != pre.id {
        return reject("ID CHANGED");
    }
    if pre.command & 2 == 0
        || post.command != 0
        || now.command != 0
        || post.bar0 != 0
        || post.bar1 != 0
        || now.bar0 != 0
        || now.bar1 != 0
    {
        return reject("NOT EBS LOSS");
    }
    if pre.bar0 & 15 != 0 || pre.bar1 != 0 || pre.bar0 == 0 {
        return reject("BAR TYPE");
    }
    if base != pre.bar0 as usize
        || base < 0x100000
        || base % 4096 != 0
        || bytes < 4096
        || !bytes.is_power_of_two()
        || base % bytes != 0
        || base.checked_add(bytes).is_none_or(|e| e > 1usize << 32)
    {
        return reject("BAR RANGE");
    }
    if [early.pm, pre.pm, post.pm, now.pm]
        .iter()
        .any(|&p| p != pre.pm)
        || (pre.pm != u32::MAX && pre.pm & 3 != 0)
        || pre.pm == u32::MAX - 1
    {
        return reject("POWER CHANGED");
    }
    Decision::Restore
}
#[cfg(test)]
mod tests {
    use super::*;
    fn before() -> State {
        State {
            id: 0x15ec8086,
            command: 7,
            bar0: 0x8f900000,
            bar1: 0,
            pm: 0,
        }
    }
    fn lost() -> State {
        State {
            command: 0,
            bar0: 0,
            ..before()
        }
    }
    fn decision(e: State, p: State, q: State, n: State, owner: bool) -> Decision {
        decide(e, p, q, n, 0x8f900000, 0x10000, owner)
    }
    #[test]
    fn unchanged_other_machine_and_measured_loss() {
        assert_eq!(
            decision(before(), before(), before(), before(), false),
            Decision::Unchanged
        );
        assert_eq!(
            decision(before(), before(), lost(), lost(), true),
            Decision::Restore
        );
        assert!(matches!(
            decision(before(), before(), lost(), lost(), false),
            Decision::Reject(_)
        ));
    }
    #[test]
    fn reject_incomplete_or_changed_evidence() {
        for n in [
            State {
                id: 0xffffffff,
                ..lost()
            },
            State {
                command: 4,
                ..lost()
            },
            State {
                bar0: 0x90000000,
                ..lost()
            },
            State { pm: 3, ..lost() },
        ] {
            assert!(matches!(
                decision(before(), before(), lost(), n, true),
                Decision::Reject(_)
            ));
        }
        for p in [
            State {
                bar0: 0x8f900004,
                ..before()
            },
            State {
                bar0: 0x8f900008,
                ..before()
            },
            State {
                bar1: 1,
                ..before()
            },
            State {
                pm: u32::MAX - 1,
                ..before()
            },
        ] {
            assert!(matches!(
                decision(p, p, lost(), lost(), true),
                Decision::Reject(_)
            ));
        }
        assert!(matches!(
            decision(lost(), before(), lost(), lost(), true),
            Decision::Reject(_)
        ));
        assert!(matches!(
            decide(
                before(),
                before(),
                lost(),
                lost(),
                0x81700000,
                0x10000,
                true
            ),
            Decision::Reject(_)
        ));
    }
}

/// Type-1 non-prefetchable memory window; inclusive hardware limit.
pub fn bridge_window(window: u32, base: usize, bytes: usize) -> bool {
    let Some(end) = base.checked_add(bytes).filter(|_| bytes > 0) else {
        return false;
    };
    let start = ((window & 0xfff0) as u64) << 16;
    let limit = ((window as u64) & 0xfff00000) | 0xfffff;
    window & 0x000f000f == 0 && start <= base as u64 && (end - 1) as u64 <= limit
}
#[cfg(test)]
mod window_tests {
    use super::*;
    #[test]
    fn forwarding_bounds_and_disabled_window() {
        assert!(bridge_window(0x8f908f90, 0x8f900000, 0x10000));
        assert!(!bridge_window(0x81708170, 0x8f900000, 0x10000));
        assert!(!bridge_window(0x8f808f90, 0x8f900000, 0x10000));
        assert!(!bridge_window(0x8f918f90, 0x8f900000, 0x10000));
        assert!(!bridge_window(0x8f908f90, 0x8f9ff000, 0x2000));
        assert!(!bridge_window(0, usize::MAX, 4096));
    }
}
