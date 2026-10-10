// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Select one source, retaining other sources' physical state for fallback.
//! Feed normalized Keyboard/Keypad usage transitions, not transport packets.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    slot: usize,
    generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Slot,
    Connected,
    Stale,
    Usage,
    Exhausted,
}
#[derive(Clone, Copy)]
struct Entry {
    generation: u64,
    connected: bool,
    keys: [bool; 256],
}
const EMPTY: Entry = Entry {
    generation: 0,
    connected: false,
    keys: [false; 256],
};
/// Lowest connected slot wins (assign external=0, internal=1 for external-first).
/// Inactive sources must continue sending transitions. Disconnect on transport
/// failure; attach starts with no held keys. Handles reject late old-session data.
pub struct Sources<const N: usize> {
    entries: [Entry; N],
    output: [bool; 256],
}
impl<const N: usize> Default for Sources<N> {
    fn default() -> Self {
        Self {
            entries: [EMPTY; N],
            output: [false; 256],
        }
    }
}
impl<const N: usize> Sources<N> {
    pub fn active(&self) -> Option<Source> {
        self.entries
            .iter()
            .position(|e| e.connected)
            .map(|slot| Source {
                slot,
                generation: self.entries[slot].generation,
            })
    }
    fn validate(&self, source: Source) -> Result<(), Error> {
        let e = self.entries.get(source.slot).ok_or(Error::Slot)?;
        if !e.connected || e.generation != source.generation {
            return Err(Error::Stale);
        }
        Ok(())
    }
    fn reconcile(&mut self, mut emit: impl FnMut(u8, bool)) {
        let next = self
            .active()
            .map_or([false; 256], |s| self.entries[s.slot].keys);
        // Release the old selection before pressing the new selection. Within
        // each phase modifiers precede ordinary keys (e.g. Shift then A).
        for down in [false, true] {
            for k in (0xe0..=0xe7).chain(4..0xe0).chain(0xe8..256) {
                if self.output[k] != next[k] && next[k] == down {
                    emit(k as u8, down);
                }
            }
        }
        self.output = next;
    }
    pub fn connect(&mut self, slot: usize, emit: impl FnMut(u8, bool)) -> Result<Source, Error> {
        let e = self.entries.get_mut(slot).ok_or(Error::Slot)?;
        if e.connected {
            return Err(Error::Connected);
        }
        let generation = e.generation.checked_add(1).ok_or(Error::Exhausted)?;
        *e = Entry {
            generation,
            connected: true,
            keys: [false; 256],
        };
        self.reconcile(emit);
        Ok(Source { slot, generation })
    }
    pub fn disconnect(&mut self, source: Source, emit: impl FnMut(u8, bool)) -> Result<(), Error> {
        self.validate(source)?;
        self.entries[source.slot].connected = false;
        self.entries[source.slot].keys = [false; 256];
        self.reconcile(emit);
        Ok(())
    }
    /// Usage 0 is padding; 1..=3 are rollover/error indicators, not keys.
    pub fn key(
        &mut self,
        source: Source,
        usage: u8,
        down: bool,
        emit: impl FnMut(u8, bool),
    ) -> Result<(), Error> {
        self.validate(source)?;
        if usage < 4 {
            return Err(Error::Usage);
        }
        self.entries[source.slot].keys[usage as usize] = down;
        self.reconcile(emit);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;
    #[test]
    fn external_preempts_and_disconnect_falls_back_without_stuck_shift() {
        let mut s = Sources::<2>::default();
        let internal = s.connect(1, |_, _| {}).unwrap();
        s.key(internal, 4, true, |_, _| {}).unwrap();
        let mut events = Vec::new();
        let external = s.connect(0, |k, d| events.push((k, d))).unwrap();
        assert_eq!(events, [(4, false)]);
        s.key(external, 0xe1, true, |_, _| {}).unwrap();
        s.key(external, 5, true, |_, _| {}).unwrap();
        s.key(internal, 6, true, |_, _| panic!("inactive source emitted"))
            .unwrap();
        events.clear();
        s.disconnect(external, |k, d| events.push((k, d))).unwrap();
        assert_eq!(events, [(0xe1, false), (5, false), (4, true), (6, true)]);
        assert_eq!(s.active(), Some(internal));
        events.clear();
        s.disconnect(internal, |k, d| events.push((k, d))).unwrap();
        assert_eq!(events, [(4, false), (6, false)]);
        assert_eq!(s.active(), None);
    }
    #[test]
    fn same_held_key_is_not_duplicated_on_fallback() {
        let mut s = Sources::<2>::default();
        let a = s.connect(0, |_, _| {}).unwrap();
        let b = s.connect(1, |_, _| {}).unwrap();
        for source in [a, b] {
            s.key(source, 4, true, |_, _| {}).unwrap();
        }
        s.key(a, 4, true, |_, _| panic!("repeat")).unwrap();
        s.disconnect(a, |_, _| panic!("duplicate transition"))
            .unwrap();
        let mut events = Vec::new();
        s.key(b, 4, false, |k, d| events.push((k, d))).unwrap();
        assert_eq!(events, [(4, false)]);
    }
    #[test]
    fn reconnect_rejects_old_session_and_invalid_operations() {
        let mut s = Sources::<1>::default();
        assert_eq!(s.connect(1, |_, _| {}), Err(Error::Slot));
        let old = s.connect(0, |_, _| {}).unwrap();
        assert_eq!(s.connect(0, |_, _| {}), Err(Error::Connected));
        for usage in 0..4 {
            assert_eq!(s.key(old, usage, true, |_, _| panic!()), Err(Error::Usage));
        }
        s.disconnect(old, |_, _| {}).unwrap();
        let new = s.connect(0, |_, _| {}).unwrap();
        assert_ne!(new, old);
        assert_eq!(s.key(old, 4, true, |_, _| panic!()), Err(Error::Stale));
        assert_eq!(s.disconnect(old, |_, _| panic!()), Err(Error::Stale));
        assert_eq!(s.active(), Some(new));
        s.disconnect(new, |_, _| {}).unwrap();
        s.entries[0].generation = u64::MAX;
        assert_eq!(s.connect(0, |_, _| {}), Err(Error::Exhausted));
    }
    #[test]
    fn boot_reports_feed_normalized_events_and_detach_after_rollover() {
        let mut sources = Sources::<1>::default();
        let source = sources.connect(0, |_, _| {}).unwrap();
        let mut decoder = crate::BootKeyboardState::default();
        let mut events = Vec::new();
        decoder.update([2, 0, 4, 0, 0, 0, 0, 0], |k, d| {
            sources
                .key(source, k, d, |k, d| events.push((k, d)))
                .unwrap()
        });
        assert_eq!(events, [(0xe1, true), (4, true)]);
        assert!(!decoder.update([0, 0, 1, 1, 1, 1, 1, 1], |_, _| panic!()));
        events.clear();
        sources
            .disconnect(source, |k, d| events.push((k, d)))
            .unwrap();
        assert_eq!(events, [(0xe1, false), (4, false)]);
    }
}
