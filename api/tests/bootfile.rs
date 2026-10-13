// SPDX-License-Identifier: Apache-2.0
use musha_api::{bootfile::*, stream::*};
fn entry(name: &str, bytes: u64) -> Entry {
    let mut e = Entry {
        name: [0; 255],
        name_len: name.len() as u8,
        bytes,
    };
    for (a, b) in e.name.iter_mut().zip(name.encode_utf16()) {
        *a = b;
    }
    e
}
#[test]
fn bounded_exact_copy_both_formats_and_case_insensitive_name() {
    for format in [Format::Fat32, Format::Exfat] {
        let entries = [entry("OTHER.ELF", 100), entry("musha.elf", 9001)];
        let data: Vec<u8> = (0..9001).map(|n| n as u8).collect();
        let mut output = vec![0xa5; 9010];
        let mut f = Files::new();
        f.mount(format, &entries).unwrap();
        let mut r = Reader::new();
        for now in 0..10 {
            r.step(&mut f, &mut output, now);
            if let Some(q) = f.take_request() {
                assert_eq!(q.index(), 1);
                assert!(q.length() <= MAX_READ);
                let at = q.offset() as usize;
                assert!(f.complete(q, Ok(&data[at..at + q.length()])));
            }
        }
        assert_eq!(r.state(), State::Ready(9001));
        assert_eq!(&output[..9001], data);
        assert_eq!(&output[9001..], &[0xa5; 9]);
        let id = f
            .list_next(&mut f.list_begin().unwrap())
            .unwrap()
            .unwrap()
            .0;
        assert!(f.open_id(id).is_ok()); // reader closed its handle
    }
}
#[test]
fn absent_duplicate_and_size_before_io() {
    for (entries, expected) in [
        (vec![entry("MUSHA.ELF.TXT", 64)], State::Absent),
        (
            vec![entry("MUSHA.ELF", 64), entry("musha.elf", 64)],
            State::Rejected(Rejection::Duplicate),
        ),
        (
            vec![entry("MUSHA.ELF", 63)],
            State::Rejected(Rejection::Size),
        ),
        (
            vec![entry("MUSHA.ELF", MAX_BYTES as u64 + 1)],
            State::Rejected(Rejection::Size),
        ),
        (
            vec![entry("MUSHA.ELF", 65)],
            State::Rejected(Rejection::Size),
        ),
    ] {
        let mut f = Files::new();
        f.mount(Format::Fat32, &entries).unwrap();
        let mut r = Reader::new();
        let mut out = [0xa5; 64];
        assert_eq!(r.step(&mut f, &mut out, 0), expected);
        assert!(f.take_request().is_none());
        assert_eq!(out, [0xa5; 64]);
    }
}
#[test]
fn short_error_disconnect_never_publish_partial_image() {
    for mode in 0..3 {
        let entries = [entry("MUSHA.ELF", 5000)];
        let mut f = Files::new();
        f.mount(Format::Exfat, &entries).unwrap();
        let mut r = Reader::new();
        let mut out = [0; 5000];
        r.step(&mut f, &mut out, 0);
        let q = f.take_request().unwrap();
        f.complete(q, Ok(&[7; 4096]));
        r.step(&mut f, &mut out, 1);
        r.step(&mut f, &mut out, 2);
        let q = f.take_request().unwrap();
        let error = match mode {
            0 => {
                f.complete(q, Ok(&[1; 10]));
                Error::Corrupt
            }
            1 => {
                f.complete(q, Err(Error::Io));
                Error::Io
            }
            _ => {
                f.disconnect();
                assert!(f.request_canceled(q));
                f.complete(q, Err(Error::Disconnected));
                Error::Disconnected
            }
        };
        assert_eq!(
            r.step(&mut f, &mut out, 3),
            State::Rejected(Rejection::Stream(error))
        );
    }
}
#[test]
fn deadline_cancels_outstanding_request_without_reusing_transport() {
    let entries = [entry("MUSHA.ELF", 64)];
    let mut f = Files::new();
    f.mount(Format::Fat32, &entries).unwrap();
    let mut r = Reader::new();
    let mut out = [0; 64];
    r.step(&mut f, &mut out, 0);
    let q = f.take_request().unwrap();
    assert_eq!(
        r.step(&mut f, &mut out, TIMEOUT_MS),
        State::Rejected(Rejection::Timeout)
    );
    assert!(f.request_canceled(q));
    assert_eq!(f.mount(Format::Fat32, &entries), Err(Error::Again));
    assert!(f.complete(q, Ok(&[1; 64])));
    assert_eq!(out, [0; 64]);
}
#[test]
fn incomplete_and_completed_images_invalidated_by_session_failure() {
    let entries = [entry("MUSHA.ELF", 64)];
    let mut f = Files::new();
    f.mount(Format::Fat32, &entries).unwrap();
    let mut r = Reader::new();
    let mut out = [0; 64];
    r.step(&mut f, &mut out, 0);
    let q = f.take_request().unwrap();
    f.complete(q, Ok(&[0; 64]));
    r.step(&mut f, &mut out, 1);
    r.interrupted();
    assert_eq!(r.state(), State::Rejected(Rejection::Interrupted));
}
