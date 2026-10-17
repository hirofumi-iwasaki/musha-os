// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! LE signaling (CID 5) after connection. Parameter values are synthetic
//! fixtures shaped like common HID peripheral requests, not a measured Xbox trace.
use musha_platform::bluetooth::{
    Error,
    acl::Reassembly,
    signaling::{Parameters, Signal, Signaling, update_complete},
};

fn request(identifier: u8, min: u16, max: u16, latency: u16, timeout: u16) -> Vec<u8> {
    let mut p = vec![0x12, identifier, 8, 0];
    for v in [min, max, latency, timeout] {
        p.extend_from_slice(&v.to_le_bytes());
    }
    p
}

#[test]
fn first_acl_on_cid5_is_reassembled_instead_of_rejected() {
    // ACL handle 0x0040, PB=10b; L2CAP length 12, CID 5; Parameter Update Request.
    let sdu = request(1, 6, 6, 0, 300);
    let mut packet = vec![
        0x40,
        0x20,
        (4 + sdu.len()) as u8,
        0,
        sdu.len() as u8,
        0,
        5,
        0,
    ];
    packet.extend_from_slice(&sdu);
    let mut r = Reassembly::new(1, 0x40).unwrap();
    let mut got = None;
    assert_eq!(
        r.receive(1, 0, &packet, |cid, p| got = Some((cid, p.to_vec()))),
        Ok(true)
    );
    assert_eq!(got, Some((5, sdu)));
    // Other dynamic CIDs remain unsupported.
    let mut other = packet.clone();
    other[6] = 0x40;
    assert_eq!(
        Reassembly::new(1, 0x40)
            .unwrap()
            .receive(1, 0, &other, |_, _| panic!()),
        Err(Error::Unsupported)
    );
}

#[test]
fn typical_hid_parameter_update_is_accepted_and_issues_connection_update() {
    let mut s = Signaling::new();
    // 7.5 ms interval, no latency, 3 s supervision timeout.
    let Signal::Update {
        identifier,
        parameters,
        response,
    } = s.receive(&request(7, 6, 6, 0, 300)).unwrap()
    else {
        panic!("not accepted")
    };
    assert_eq!(identifier, 7);
    assert_eq!(response, [0x13, 7, 2, 0, 0, 0]); // accepted
    assert_eq!(
        parameters.command(0x0040),
        [0x40, 0, 6, 0, 6, 0, 0, 0, 0x2c, 1, 0, 0, 0, 0]
    );
    // Wider range with latency, as keyboards often request.
    assert!(matches!(
        s.receive(&request(8, 12, 24, 4, 400)).unwrap(),
        Signal::Update { .. }
    ));
}

#[test]
fn out_of_range_requests_are_rejected_but_answered() {
    for (min, max, latency, timeout) in [
        (5, 6, 0, 300),    // interval below 7.5 ms
        (6, 3201, 0, 300), // interval above 4 s
        (24, 12, 0, 300),  // min > max
        (6, 6, 500, 3200), // latency above 499
        (6, 6, 0, 9),      // timeout below 100 ms
        (6, 6, 0, 3201),   // timeout above 32 s
        (80, 80, 4, 100),  // timeout not above (1+latency)*max*2
    ] {
        let p = Parameters {
            min,
            max,
            latency,
            timeout,
        };
        assert!(!p.valid(), "{p:?}");
        let mut s = Signaling::new();
        match s.receive(&request(3, min, max, latency, timeout)).unwrap() {
            Signal::Rejected { response, .. } => assert_eq!(response, [0x13, 3, 2, 0, 1, 0]),
            other => panic!("{other:?}"),
        }
    }
    // Boundary: 4 s timeout limit exactly satisfied is valid.
    assert!(
        Parameters {
            min: 6,
            max: 3200,
            latency: 0,
            timeout: 3200
        }
        .valid()
    );
}

#[test]
fn unknown_requests_get_command_reject_and_responses_are_never_answered() {
    let mut s = Signaling::new();
    // LE Credit Based Connection Request: unsupported request.
    let mut credit = vec![0x14, 9, 10, 0];
    credit.extend_from_slice(&[0x80, 0, 0x40, 0, 23, 0, 23, 0, 1, 0]);
    assert_eq!(
        s.receive(&credit).unwrap(),
        Signal::Reject {
            identifier: 9,
            code: 0x14,
            response: [0x01, 9, 2, 0, 0, 0]
        }
    );
    // A Parameter Update Request with a wrong length is not understood.
    let mut short = request(10, 6, 6, 0, 300);
    short.truncate(10);
    short[2] = 6;
    assert!(matches!(s.receive(&short).unwrap(), Signal::Reject { .. }));
    // Command Reject, responses and Flow Control Credit are ignored.
    for code in [0x01, 0x13, 0x15, 0x16] {
        assert_eq!(
            s.receive(&[code, 11, 2, 0, 0, 0]).unwrap(),
            Signal::Ignore {
                identifier: 11,
                code,
                duplicate: false
            }
        );
    }
    // Malformed envelope and identifier 0 fail closed.
    assert_eq!(s.receive(&[0x12, 1, 8]), Err(Error::Length));
    assert_eq!(s.receive(&[0x12, 1, 9, 0, 0]), Err(Error::Length));
    assert_eq!(s.receive(&request(0, 6, 6, 0, 300)), Err(Error::Invalid));
}

#[test]
fn a_repeated_identifier_is_answered_only_once() {
    let mut s = Signaling::new();
    assert!(matches!(
        s.receive(&request(4, 6, 6, 0, 300)).unwrap(),
        Signal::Update { .. }
    ));
    assert_eq!(
        s.receive(&request(4, 6, 6, 0, 300)).unwrap(),
        Signal::Ignore {
            identifier: 4,
            code: 0x12,
            duplicate: true
        }
    );
    assert!(matches!(
        s.receive(&request(5, 6, 6, 0, 300)).unwrap(),
        Signal::Update { .. }
    ));
}

#[test]
fn connection_update_complete_metadata() {
    let p = [3, 0, 0x40, 0, 6, 0, 0, 0, 0x2c, 1];
    let u = update_complete(&p).unwrap();
    assert_eq!(
        (u.status, u.handle, u.interval, u.latency, u.timeout),
        (0, 0x40, 6, 0, 300)
    );
    assert_eq!(update_complete(&p[..9]), None);
    assert_eq!(update_complete(&[1, 0, 0x40, 0, 6, 0, 0, 0, 0x2c, 1]), None);
}
