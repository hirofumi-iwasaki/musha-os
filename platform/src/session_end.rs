// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Metadata-only common session termination trace. No device mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct End {
    pub failed: bool,
    pub trigger: &'static str,
    pub reason: &'static str,
}
pub struct Trace {
    requested: Option<&'static str>,
    end: Option<End>,
}
impl Trace {
    pub const fn new() -> Self {
        Self {
            requested: None,
            end: None,
        }
    }
    pub fn request(&mut self, reason: &'static str) {
        if self.requested.is_none() {
            self.requested = Some(reason);
        }
    }
    pub fn finish(&mut self, result: Result<(), &'static str>) -> End {
        let trigger = self.requested.unwrap_or(if result.is_err() {
            "no explicit stop request recorded"
        } else {
            "USB loop completed; no stop request recorded"
        });
        let end = End {
            failed: result.is_err(),
            trigger,
            reason: result.err().unwrap_or(trigger),
        };
        self.end = Some(end);
        end
    }
    pub fn end(&self) -> Option<End> {
        self.end
    }
}
pub fn stop_request(quit: bool, complete: bool, restart: bool) -> Option<&'static str> {
    if quit {
        Some("application quit requested")
    } else if restart {
        Some("Bluetooth restart requested")
    } else if complete {
        Some("application step completed")
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_common_results_keep_request_and_error_independently() {
        let mut t = Trace::new();
        assert_eq!(t.end(), None);
        t.request("quit");
        t.request("later");
        let e = t.finish(Ok(()));
        assert!(!e.failed);
        assert_eq!((e.trigger, e.reason), ("quit", "quit"));
        assert_eq!(t.end(), Some(e));
        let e = t.finish(Err("BAD TRANSFER"));
        assert!(e.failed);
        assert_eq!((e.trigger, e.reason), ("quit", "BAD TRANSFER"));
        let mut t = Trace::new();
        let e = t.finish(Ok(()));
        assert!(!e.failed);
        assert!(e.reason.contains("no stop request"));
        let e = t.finish(Err("TIMEOUT"));
        assert!(e.failed);
        assert_eq!(e.reason, "TIMEOUT");
    }
    #[test]
    fn callbacks_use_production_priority_without_changing_stop_boolean() {
        for q in [false, true] {
            for c in [false, true] {
                for r in [false, true] {
                    let s = stop_request(q, c, r);
                    assert_eq!(s.is_some(), q || c || r);
                }
            }
        }
        assert_eq!(
            stop_request(false, true, true),
            Some("Bluetooth restart requested")
        );
        assert_eq!(
            stop_request(true, true, true),
            Some("application quit requested")
        );
    }
}
