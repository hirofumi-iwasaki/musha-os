// SPDX-License-Identifier: BSD-2-Clause
use musha_bce::{Error, wire::*};
#[test]
fn mailbox_golden_words_and_bounds() {
    let msg = Message::new(SET_PROTOCOL, PROTOCOL_VERSION).unwrap();
    assert_eq!(msg.raw(), 0x3000000000020001);
    assert_eq!(msg.words(), [0x00020001, 0x30000000, 0, 0]);
    assert_eq!(Message::new(64, 0), Err(Error::Invalid));
    assert_eq!(Message::new(0, 1 << 58), Err(Error::Invalid));
    assert_eq!(Message::new(63, (1 << 58) - 1).unwrap().raw(), u64::MAX);
}
#[test]
fn queue_and_submission_golden_layouts() {
    assert_eq!(
        queue_memory(2, 4, 1, 0x12345000, 32).unwrap(),
        [
            2, 0, 4, 0, 1, 0, 0, 0, 0, 0x50, 0x34, 0x12, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0
        ]
    );
    assert_eq!(
        submission(0x12345000, 8).unwrap(),
        [
            8, 0, 0, 0, 0, 0, 0, 0, 0, 0x50, 0x34, 0x12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0
        ]
    );
    for (addr, len) in [(0, 8), (1, 8), (4, 0), (u64::MAX - 3, 8)] {
        assert!(submission(addr, len).is_err());
    }
    for (qid, count, stride) in [(256, 4, 32), (2, 1, 32), (2, 257, 32), (2, 4, 7)] {
        assert!(queue_memory(qid, count, 1, 4096, stride).is_err());
    }
}
#[test]
fn completion_golden_and_all_truncations() {
    let b = [
        9, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 2, 0, 3, 0, 0, 0, 0, 128,
    ];
    assert_eq!(
        Completion::decode(&b),
        Ok(Completion {
            result: 9,
            data_size: 8,
            qid: 2,
            index: 3,
            status: 0,
            flags: PENDING
        })
    );
    for n in 0..24 {
        assert_eq!(Completion::decode(&b[..n]), Err(Error::Length));
    }
    assert_eq!(Completion::decode(&[0; 25]), Err(Error::Length));
}
