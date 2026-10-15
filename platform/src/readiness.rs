// SPDX-License-Identifier: Apache-2.0
//! Internal static/cooperative profile. No external ABI or Ring 3 claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Cold,
    BootFailed,
    CoreReady,
    OsReady,
    AppEntering,
    AppRunning,
    AppExited,
    AppFailed,
    AppLaunchFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    Unavailable,
    Live,
    Stopped,
    StopUnconfirmed,
}
#[derive(Clone, Copy, Debug)]
pub struct Report {
    pub phase: Phase,
    pub core_ms: Option<u64>,
    pub ready_ms: Option<u64>,
    pub entry_ms: Option<u64>,
    pub running_ms: Option<u64>,
    pub first_frame_ms: Option<u64>,
    pub usb: Service,
    pub network: Service,
    pub files: Service,
}
impl Report {
    pub const NEW: Self = Self {
        phase: Phase::Cold,
        core_ms: None,
        ready_ms: None,
        entry_ms: None,
        running_ms: None,
        first_frame_ms: None,
        usb: Service::Unavailable,
        network: Service::Unavailable,
        files: Service::Unavailable,
    };
    pub fn core(&mut self, now: Option<u64>) -> bool {
        if self.phase != Phase::Cold {
            return false;
        }
        self.phase = Phase::CoreReady;
        self.core_ms = now;
        true
    }
    pub fn prepare_launch(&mut self, now: Option<u64>) -> bool {
        if !matches!(
            self.phase,
            Phase::AppRunning | Phase::AppExited | Phase::AppFailed | Phase::AppLaunchFailed
        ) {
            return false;
        }
        self.phase = Phase::OsReady;
        self.ready_ms = now;
        self.entry_ms = None;
        self.running_ms = None;
        self.first_frame_ms = None;
        true
    }
    pub fn restart(&mut self) {
        self.phase = Phase::CoreReady;
        self.ready_ms = None;
        self.entry_ms = None;
        self.running_ms = None;
        self.first_frame_ms = None;
        self.usb = Service::Stopped;
        self.files = Service::Stopped;
        self.network = Service::Stopped;
    }
    pub fn ready(&mut self, required: bool, now: Option<u64>) -> bool {
        if !required || self.phase != Phase::CoreReady {
            return false;
        }
        self.phase = Phase::OsReady;
        self.ready_ms = now;
        true
    }
    pub fn entering(&mut self, now: Option<u64>) -> bool {
        if self.phase != Phase::OsReady {
            return false;
        }
        self.phase = Phase::AppEntering;
        self.entry_ms = now;
        true
    }
    pub fn running(&mut self, now: Option<u64>) -> bool {
        if self.phase != Phase::AppEntering {
            return false;
        }
        self.phase = Phase::AppRunning;
        self.running_ms = now;
        true
    }
    pub fn frame(&mut self, now: Option<u64>) {
        if self.phase == Phase::AppRunning && self.first_frame_ms.is_none() {
            self.first_frame_ms = now;
        }
    }
    pub fn finish(&mut self, success: bool) {
        self.phase = if self.phase != Phase::AppRunning {
            Phase::AppLaunchFailed
        } else if success {
            Phase::AppExited
        } else {
            Phase::AppFailed
        };
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_and_selected_launch_preserve_boot_epoch_and_clear_app_boundaries() {
        let mut report = Report::NEW;
        assert!(!report.prepare_launch(Some(1)));
        report.core(Some(2));
        report.ready(true, Some(3));
        report.entering(Some(4));
        report.running(Some(5));
        report.frame(Some(6));
        assert!(report.prepare_launch(Some(7)));
        assert_eq!(report.core_ms, Some(2));
        assert_eq!(report.entry_ms, None);
        assert_eq!(report.first_frame_ms, None);
        report.restart();
        assert_eq!(report.phase, Phase::CoreReady);
        assert_eq!(report.usb, Service::Stopped);
        assert_eq!(report.files, Service::Stopped);
        assert_eq!(report.core_ms, Some(2));
        assert!(report.ready(true, Some(8)));
    }
    #[test]
    fn ordering_and_required_failure() {
        let mut r = Report::NEW;
        assert!(!r.ready(true, Some(1)));
        assert!(r.core(None));
        assert!(!r.ready(false, Some(2)));
        assert!(r.ready(true, Some(3)));
        assert!(!r.ready(true, Some(4)));
        assert_eq!(r.ready_ms, Some(3));
        assert!(r.entering(Some(4)));
        assert!(r.running(Some(5)));
        r.frame(Some(6));
        r.frame(Some(7));
        assert_eq!(r.first_frame_ms, Some(6));
        r.usb = Service::Stopped;
        assert_eq!(r.phase, Phase::AppRunning);
        r.finish(true);
        assert_eq!(r.phase, Phase::AppExited);
    }
    #[test]
    fn headless_and_failed_launch() {
        let mut r = Report::NEW;
        r.core(None);
        r.ready(true, None);
        r.entering(None);
        r.finish(false);
        assert_eq!(r.phase, Phase::AppLaunchFailed);
        assert_eq!(r.first_frame_ms, None);
    }
}
