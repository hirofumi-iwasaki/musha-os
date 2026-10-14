// SPDX-License-Identifier: Apache-2.0
//! Bounded SuperSpeed root attachment policy. No hardware I/O here.
// Engineering margins, not mandated delays: Linux v6.12 hub.h uses >=100 ms
// for NON-root hub power-good (root hubs excepted); hub.c debounce is 100 ms.
// Routed grace exceeds the review's USB3 tPollingLFPSTimeout=360 ms;
// 500 ms remains provisional pending cold NUC5 routing-to-U0 captures.
pub const NORMAL_GRACE_MS: u64 = 150;
pub const ROUTED_GRACE_MS: u64 = 500;
pub const fn grace_ms(wrote_routing: bool) -> u64 {
    if wrote_routing {
        ROUTED_GRACE_MS
    } else {
        NORMAL_GRACE_MS
    }
}
pub const fn rescan_allowed(
    keyboard: bool,
    readable: bool,
    attempted_device: bool,
    scans: u8,
) -> bool {
    !keyboard && !readable && !attempted_device && scans == 0
}
pub const BUDGET_MS: u64 = 2000;
pub const STUCK_MS: u64 = 200;
pub const MAX_RESETS: u8 = 3;
pub const POLL_MS: u64 = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Wait,
    Ready,
    Empty,
    WarmReset,
    Expired,
}
#[derive(Clone, Copy)]
pub struct Attach {
    pub attempts: u8,
    last_pls: u8,
    since: u64,
}
impl Attach {
    pub const fn new() -> Self {
        Self {
            attempts: 0,
            last_pls: 255,
            since: 0,
        }
    }
    pub fn sample(&mut self, status: u32, elapsed: u64, since_power: u64, grace: u64) -> Action {
        let pls = ((status >> 5) & 15) as u8;
        if empty(status) {
            return if since_power < grace {
                Action::Wait
            } else {
                Action::Empty
            };
        }
        if ready(status) {
            return Action::Ready;
        }
        if elapsed >= BUDGET_MS {
            return Action::Expired;
        }
        if pls != self.last_pls {
            self.last_pls = pls;
            self.since = elapsed;
        }
        // Never restart an active reset, unpowered port, or overcurrent port.
        if status & 1 != 0
            && matches!(pls, 6 | 7 | 10)
            && status & (1 << 9) != 0
            && status & ((1 << 31) | (1 << 4) | (1 << 3)) == 0
            && elapsed.saturating_sub(self.since) >= STUCK_MS
            && self.attempts < MAX_RESETS
        {
            self.attempts += 1;
            self.since = elapsed;
            return Action::WarmReset;
        }
        Action::Wait
    }
}
pub fn empty(status: u32) -> bool {
    status & 1 == 0 && (status >> 5) & 15 == 5
}
pub fn ready(status: u32) -> bool {
    status & 3 == 3 && (status >> 5) & 15 == 0 && status & ((1 << 31) | (1 << 4) | (1 << 3)) == 0
}
pub fn warm_reset(status: u32) -> u32 {
    // Preserve only neutral control bits. Never replay PED, W1C or LWS.
    crate::port_control(status) | (1 << 31)
}
#[cfg(test)]
mod tests {
    use super::*;
    const PP: u32 = 1 << 9;
    #[test]
    fn grace_table_and_rescan_gate() {
        for routed in [false, true] {
            let grace = grace_ms(routed);
            assert_eq!(grace, if routed { 500 } else { 150 });
            for (status, power, expected) in [
                (PP | (5 << 5), 0, Action::Wait),
                (PP | (5 << 5), grace - 1, Action::Wait),
                (PP | (5 << 5), grace, Action::Empty),
                (PP | 1 | (7 << 5), 0, Action::Wait),
                (PP | 3 | (4 << 10), 0, Action::Ready),
                (PP | (7 << 5), grace, Action::Wait),
            ] {
                assert_eq!(Attach::new().sample(status, 0, power, grace), expected);
            }
        }
        for k in [false, true] {
            for s in [false, true] {
                for d in [false, true] {
                    for n in [0, 1, 2] {
                        assert_eq!(rescan_allowed(k, s, d, n), !k && !s && !d && n == 0);
                    }
                }
            }
        }
    }
    #[test]
    fn empty_ports_skip_wait_and_disconnected_stuck_links_never_reset() {
        let mut a = Attach::new();
        assert_eq!(a.sample(PP | (5 << 5), 0, 500, 150), Action::Empty);
        for pls in [6, 7, 10] {
            let mut a = Attach::new();
            a.sample(PP | (pls << 5), 0, 500, 150);
            assert_eq!(a.sample(PP | (pls << 5), 500, 500, 150), Action::Wait);
            assert_eq!(a.sample(PP | (pls << 5), 2000, 500, 150), Action::Expired);
            assert_eq!(a.attempts, 0);
        }
        assert_eq!(a.sample(PP | 1 | (5 << 5), 0, 500, 150), Action::Wait);
    }
    #[test]
    fn transient_polling_reaches_u0_without_reset() {
        let mut a = Attach::new();
        assert_eq!(a.sample(PP | 1 | (7 << 5), 0, 500, 150), Action::Wait);
        assert_eq!(a.sample(PP | 3 | (4 << 10), 100, 500, 150), Action::Ready);
        assert_eq!(a.attempts, 0);
    }
    #[test]
    fn stuck_states_have_three_resets_and_shared_deadline() {
        for pls in [6, 7, 10] {
            let mut a = Attach::new();
            let s = PP | 1 | (pls << 5);
            assert_eq!(a.sample(s, 0, 500, 150), Action::Wait);
            for t in [200, 400, 600] {
                assert_eq!(a.sample(s, t, 500, 150), Action::WarmReset);
            }
            assert_eq!(a.sample(s, 800, 500, 150), Action::Wait);
            assert_eq!(a.sample(s, 2000, 500, 150), Action::Expired);
            assert_eq!(a.attempts, 3);
        }
    }
    #[test]
    fn reset_busy_overcurrent_unpowered_and_rxdetect_are_not_reset() {
        for s in [
            PP | 1 | (5 << 5),
            PP | (7 << 5) | (1 << 31),
            PP | (7 << 5) | (1 << 4),
            PP | (7 << 5) | 8,
            7 << 5,
        ] {
            let mut a = Attach::new();
            a.sample(s, 0, 500, 150);
            assert_eq!(a.sample(s, 1000, 500, 150), Action::Wait);
            assert_eq!(a.sample(s, 2000, 500, 150), Action::Expired);
            assert_eq!(a.attempts, 0);
        }
    }
    #[test]
    fn stuck_timer_restarts_when_link_state_changes() {
        let mut a = Attach::new();
        a.sample(PP | 1 | (7 << 5), 0, 500, 150);
        assert_eq!(a.sample(PP | 1 | (6 << 5), 190, 500, 150), Action::Wait);
        assert_eq!(a.sample(PP | 1 | (6 << 5), 200, 500, 150), Action::Wait);
        assert_eq!(
            a.sample(PP | 1 | (6 << 5), 390, 500, 150),
            Action::WarmReset
        );
    }
    #[test]
    fn ready_requires_enabled_connected_u0_and_no_active_reset() {
        for s in [0, 1, 2, 3 | (7 << 5), 3 | (1 << 4), 3 | (1 << 31), 3 | 8] {
            assert!(!ready(s));
        }
        assert!(ready(PP | 3 | (4 << 10)));
    }
    #[test]
    fn warm_reset_does_not_replay_change_or_disable_bits() {
        let written = warm_reset(u32::MAX);
        assert_eq!(written & ((1 << 1) | (1 << 16) | 0xfe0000 | (1 << 4)), 0);
        assert_ne!(written & (1 << 31), 0);
        assert_ne!(written & PP, 0);
    }
}
