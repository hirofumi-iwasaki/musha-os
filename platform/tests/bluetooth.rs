// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::{
    self as bt, Acl, Error, Event,
    scan::{Scan, State},
    usb,
};
fn config() -> Vec<u8> {
    vec![
        9, 2, 39, 0, 1, 1, 0, 0x80, 50, 9, 4, 0, 0, 3, 0xe0, 1, 1, 0, 7, 5, 0x81, 3, 16, 0, 1, 7,
        5, 0x82, 2, 64, 0, 0, 7, 5, 2, 2, 64, 0, 0,
    ]
}
fn complete(op: u16, status: u8, credits: u8) -> [u8; 6] {
    [14, 4, credits, op as u8, (op >> 8) as u8, status]
}
fn advert() -> Vec<u8> {
    vec![0x3e, 15, 2, 1, 0, 1, 1, 2, 3, 4, 5, 6, 3, 2, 1, 6, 0xc0]
}
#[test]
fn descriptor_selects_three_distinct_endpoints_and_nonzero_interface() {
    let mut b = config();
    b[11] = 4;
    let t = usb::select(&b).unwrap();
    assert_eq!(t.interface, 4);
    assert_eq!(
        (t.event.address, t.acl_in.address, t.acl_out.address),
        (0x81, 0x82, 2)
    );
}
#[test]
fn descriptor_rejects_every_truncation_and_malformed_tail() {
    let b = config();
    for n in 0..b.len() {
        assert!(usb::select(&b[..n]).is_err());
    }
    let mut b = b;
    b.extend([0, 5]);
    b[2] = b.len() as u8;
    assert_eq!(usb::select(&b), Err(Error::Length));
}
#[test]
fn descriptor_rejects_wrong_class_missing_duplicate_or_invalid_endpoint() {
    for (at, value) in [
        (14, 3),
        (13, 2),
        (20, 0),
        (21, 1),
        (22, 0),
        (23, 0x20),
        (24, 0),
        (27, 0x81),
        (28, 3),
    ] {
        let mut b = config();
        b[at] = value;
        assert!(usb::select(&b).is_err(), "{at}");
    }
}
#[test]
fn completed_primary_wins_over_later_hci_interface() {
    let mut b = config();
    let first = usb::select(&b).unwrap();
    let mut later = config()[9..].to_vec();
    later[2] = 1;
    b.extend_from_slice(&later);
    b[2] = b.len() as u8;
    b[4] = 2;
    assert_eq!(usb::select(&b), Ok(first));
}

#[test]
fn scan_fixture_from_descriptor_through_commands_to_ad_fields() {
    usb::select(&config()).unwrap();
    let mut s = Scan::new(9);
    for (i, op) in [0x0c03u16, 0x0c01, 0x2001, 0x200b, 0x200c]
        .into_iter()
        .enumerate()
    {
        let c = s.next(9, i as u64 * 10).unwrap().unwrap();
        assert_eq!(&c.bytes[..2], &op.to_le_bytes());
        assert_eq!(c.length, 3 + c.bytes[2] as usize);
        assert_eq!(s.next(9, i as u64 * 10), Err(Error::Busy));
        s.event(9, i as u64 * 10 + 1, &complete(op, 0, 1)).unwrap();
    }
    assert_eq!(s.state(), State::Scanning);
    assert_eq!(s.next(9, 60), Ok(None));
    let b = advert();
    s.event(9, 60, &b).unwrap();
    let mut found = 0;
    bt::advertisements(Event::parse(&b).unwrap(), |a| {
        assert_eq!(a.address, [1, 2, 3, 4, 5, 6]);
        assert_eq!(a.rssi, Some(-64));
        bt::ad_fields(a.data, |f| {
            assert_eq!(f.kind, 1);
            assert_eq!(f.value, [6]);
            found += 1;
        })
        .unwrap();
    })
    .unwrap();
    assert_eq!(found, 1);
}
#[test]
fn command_credit_zero_unsolicited_replenishment_and_startup_deadline() {
    let mut s = Scan::new(1);
    s.next(1, 0).unwrap();
    s.event(1, 1, &complete(0x0c03, 0, 0)).unwrap();
    assert_eq!(s.next(1, 2), Err(Error::Busy));
    s.event(1, 3, &[14, 3, 1, 0, 0]).unwrap();
    assert!(s.next(1, 4).unwrap().is_some());
    let mut s = Scan::new(1);
    s.event(1, 0, &[15, 4, 0, 0, 0, 0]).unwrap();
    assert_eq!(s.next(1, 1), Err(Error::Busy));
    assert_eq!(s.next(1, 5000), Err(Error::Timeout));
}
#[test]
fn scan_timeout_status_hardware_error_and_no_false_success() {
    let mut s = Scan::new(1);
    s.next(1, 100).unwrap();
    assert_eq!(s.next(1, 1100), Err(Error::Timeout));
    assert_eq!(s.state(), State::Failed);
    let mut s = Scan::new(1);
    s.next(1, 0).unwrap();
    assert_eq!(
        s.event(1, 1, &complete(0x0c03, 0x0c, 1)),
        Err(Error::Controller(0x0c))
    );
    let mut s = Scan::new(1);
    assert_eq!(s.event(1, 0, &[16, 1, 3]), Err(Error::Controller(3)));
    let mut s = Scan::new(1);
    s.next(1, 0).unwrap();
    assert_eq!(
        s.event(1, 1, &[15, 4, 0, 1, 3, 12]),
        Err(Error::Controller(0))
    );
    assert_eq!(s.state(), State::Failed);
}
#[test]
fn late_wrong_generation_opcode_and_detach_cannot_advance() {
    let mut s = Scan::new(2);
    s.next(2, 0).unwrap();
    assert_eq!(s.event(1, 1, &complete(0x0c03, 0, 1)), Err(Error::Stale));
    assert_eq!(s.event(2, 1, &complete(0x200c, 0, 1)), Err(Error::Stale));
    assert_eq!(s.state(), State::Initializing);
    s.detach();
    assert_eq!(s.event(2, 2, &complete(0x0c03, 0, 1)), Err(Error::Stale));
}
#[test]
fn monotonic_clock_and_overflow_fail_closed() {
    let mut s = Scan::new(1);
    s.next(1, 20).unwrap();
    assert_eq!(s.next(1, 19), Err(Error::Invalid));
    assert_eq!(Scan::new(1).next(1, u64::MAX), Err(Error::Invalid));
}
#[test]
fn event_framing_and_advert_truncation_never_emit_partial_report() {
    let b = advert();
    for n in 0..b.len() {
        assert!(Event::parse(&b[..n]).is_err());
    }
    let mut b = b;
    b[3] = 2;
    let mut calls = 0;
    assert!(bt::advertisements(Event::parse(&b).unwrap(), |_| calls += 1).is_err());
    assert_eq!(calls, 0);
    let mut b = advert();
    b[4] = 5;
    assert_eq!(
        bt::advertisements(Event::parse(&b).unwrap(), |_| panic!()),
        Err(Error::Unsupported)
    );
}
#[test]
fn multiple_reports_and_rssi_unavailable() {
    let mut b = advert();
    let report = b[4..].to_vec();
    b.extend(report);
    b[1] = (b.len() - 2) as u8;
    b[3] = 2;
    *b.last_mut().unwrap() = 127;
    let mut count = 0;
    bt::advertisements(Event::parse(&b).unwrap(), |a| {
        if count == 1 {
            assert_eq!(a.rssi, None);
        }
        count += 1;
    })
    .unwrap();
    assert_eq!(count, 2);
    *b.last_mut().unwrap() = 21;
    assert_eq!(
        bt::advertisements(Event::parse(&b).unwrap(), |_| panic!()),
        Err(Error::Invalid)
    );
}
#[test]
fn advertising_data_validates_whole_input_and_zero_padding() {
    for b in [&[2, 1][..], &[0, 1][..], &[1, 1, 3, 2][..]] {
        assert!(bt::ad_fields(b, |_| panic!()).is_err());
    }
    let mut n = 0;
    bt::ad_fields(&[2, 1, 6, 0, 0], |_| n += 1).unwrap();
    assert_eq!(n, 1);
}
#[test]
fn acl_fixed_channel_att_smp_bounds_and_unsupported_fragments() {
    let b = [1, 0x20, 5, 0, 1, 0, 4, 0, 0x0a];
    let a = Acl::parse(&b).unwrap();
    assert_eq!(a.handle, 1);
    assert_eq!(a.fixed_channel(), Ok((4, &[10][..])));
    let mut b = b;
    b[6] = 6;
    assert_eq!(Acl::parse(&b).unwrap().fixed_channel(), Ok((6, &[10][..])));
    for n in 0..b.len() {
        assert!(Acl::parse(&b[..n]).is_err());
    }
    b[1] = 0x10;
    assert_eq!(
        Acl::parse(&b).unwrap().fixed_channel(),
        Err(Error::Unsupported)
    );
    b[1] = 0x20;
    b[4] = 2;
    assert_eq!(Acl::parse(&b).unwrap().fixed_channel(), Err(Error::Length));
    b[1] = 0xc0;
    assert_eq!(Acl::parse(&b), Err(Error::Invalid));
}
#[test]
fn arbitrary_bounded_bytes_never_panic_or_publish_invalid_ad_fields() {
    let mut seed = 0x12345678u32;
    for n in 0..260 {
        let mut b = vec![0; n];
        for byte in &mut b {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *byte = seed as u8;
        }
        let _ = usb::select(&b);
        let _ = Acl::parse(&b).and_then(|a| a.fixed_channel());
        if let Ok(e) = Event::parse(&b) {
            let _ = bt::advertisements(e, |_| ());
        }
        let _ = bt::ad_fields(&b, |_| ());
    }
}

#[test]
fn command_control_setup_targets_device_or_composite_interface_without_h4() {
    let mut b = config();
    b[11] = 4;
    let t = usb::select(&b).unwrap();
    assert_eq!(
        t.command_setup(&[3, 12, 0], false),
        Ok([0x20, 0, 0, 0, 0, 0, 3, 0])
    );
    assert_eq!(
        t.command_setup(&[3, 12, 0], true),
        Ok([0x21, 0, 0, 0, 4, 0, 3, 0])
    );
    assert_eq!(t.command_setup(&[1, 3, 12, 0], false), Err(Error::Length));
}

#[test]
fn descriptor_accepts_idle_sco_interface_without_activating_iso_endpoints() {
    let mut b = config();
    b.extend([9, 4, 1, 0, 0, 0xe0, 1, 1, 0]);
    b.extend([
        9, 4, 1, 1, 2, 0xe0, 1, 1, 0, 7, 5, 0x83, 1, 17, 0, 1, 7, 5, 3, 1, 17, 0, 1,
    ]);
    b[2] = b.len() as u8;
    b[4] = 2;
    let t = usb::select(&b).unwrap();
    assert_eq!(t.interface, 0);
    assert_eq!(t.acl_in.address, 0x82);
}
