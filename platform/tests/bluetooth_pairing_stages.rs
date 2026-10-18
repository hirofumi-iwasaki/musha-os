// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
//! Host models of the first high-rate pairing paths: LE scan event delivery,
//! connection metadata and the first ACL/L2CAP/SMP exchange. Synthetic bytes;
//! no physical controller timing or firmware behaviour is claimed.
use musha_platform::bluetooth::{
    Event,
    acl::{Credits, Reassembly},
    advertisements,
    stage::{self, Advertising},
    stream::Stream,
    usb_context::event_transfer_bytes,
};

/// One legacy LE Advertising Report event with `reports` reports of `data` bytes.
fn advertising_event(reports: usize, data: usize, seed: u8) -> Vec<u8> {
    let mut p = vec![0x02, reports as u8];
    for r in 0..reports {
        p.extend_from_slice(&[0, 0]); // ADV_IND, public
        p.extend_from_slice(&[seed, r as u8, 3, 4, 5, 6]);
        p.push(data as u8);
        p.extend(std::iter::repeat_n(0, data)); // zero padding is valid AD data
        p.push(0xc4); // RSSI -60 dBm
    }
    let mut e = vec![0x3e, p.len() as u8];
    e.extend_from_slice(&p);
    e
}

/// Device side: every event starts on a packet boundary, its last packet is
/// short, and no ZLP follows an event that fills whole packets. Host side: one
/// interrupt IN TD of `td` bytes completes on a short packet or when full; a
/// packet longer than the remaining TD space is Babble (xHCI cc=3).
fn deliver(td: usize, max_packet: usize, events: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, usize> {
    let mut packets = vec![];
    for e in events {
        packets.extend(e.chunks(max_packet).map(<[u8]>::to_vec));
    }
    let mut transfers = vec![];
    let mut current = vec![];
    for (index, packet) in packets.iter().enumerate() {
        if packet.len() > td - current.len() {
            return Err(index);
        }
        current.extend_from_slice(packet);
        if packet.len() < max_packet || current.len() == td {
            transfers.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        transfers.push(current);
    }
    Ok(transfers)
}

#[test]
fn event_td_length_is_a_whole_number_of_interrupt_packets() {
    for (max_packet, bytes) in [(16, 256), (64, 256), (32, 256), (10, 250), (8, 256)] {
        assert_eq!(event_transfer_bytes(max_packet), bytes);
        assert_eq!(bytes % max_packet as u32, 0);
    }
    // Degenerate descriptors keep one whole HCI event per TD.
    assert_eq!(event_transfer_bytes(0), 257);
    assert_eq!(event_transfer_bytes(512), 257);
}

#[test]
fn whole_packet_events_without_zlp_babble_a_257_byte_td_but_not_the_aligned_td() {
    // 32-byte events (18 bytes of AD data) fill two whole 16-byte packets each.
    let events: Vec<_> = (0..12).map(|i| advertising_event(1, 18, i)).collect();
    assert!(events.iter().all(|e| e.len() == 32));
    // The former TD: 256 bytes of whole packets leave 1 byte for a 16-byte packet.
    assert_eq!(deliver(257, 16, &events), Err(16));
    let transfers = deliver(event_transfer_bytes(16) as usize, 16, &events).unwrap();
    assert!(transfers.iter().all(|t| t.len() <= 256));
    let mut stream = Stream::<257>::new(7, false);
    let mut seen = 0;
    for t in &transfers {
        let mut input = &t[..];
        while let Some(frame) = stream.next(7, 0, &mut input).unwrap() {
            let event = Event::parse(frame).unwrap();
            advertisements(event, |a| {
                assert_eq!(a.address[1], 0);
                seen += 1;
            })
            .unwrap();
            stream.release();
        }
    }
    assert_eq!(seen, 12);
}

#[test]
fn continuous_mixed_advertising_reports_are_framed_across_tds() {
    // Mixed lengths, including events ending exactly on packet boundaries, and
    // multi-report events up to the 255-byte parameter limit.
    let mut events = vec![];
    let mut expected = 0;
    for i in 0..400usize {
        let (reports, data) = match i % 7 {
            0 => (1, 2),  // 16 bytes
            1 => (1, 18), // 32 bytes
            2 => (1, 31),
            3 => (3, 31), // 125 bytes
            4 => (6, 31), // 248 bytes
            5 => (1, 0),
            _ => (2, 17),
        };
        let e = advertising_event(reports, data, i as u8);
        assert!(e.len() <= 257);
        expected += reports;
        events.push(e);
    }
    let td = event_transfer_bytes(16) as usize;
    let transfers = deliver(td, 16, &events).unwrap();
    let mut stream = Stream::<257>::new(3, false);
    let mut window = Advertising::new();
    let mut reports = 0;
    let mut summaries = vec![];
    for (n, t) in transfers.iter().enumerate() {
        let now = n as u64 * 5; // a TD every 5 ms
        stream.check(3, now).unwrap();
        let mut input = &t[..];
        while let Some(frame) = stream.next(3, now, &mut input).unwrap() {
            let event = Event::parse(frame).unwrap();
            window.event(now);
            advertisements(event, |a| {
                window.report(&a.address);
                reports += 1;
            })
            .unwrap();
            stream.release();
        }
        summaries.extend(window.due(now));
    }
    assert_eq!(reports, expected);
    // Aggregated roughly once per second, never per report.
    assert!(!summaries.is_empty() && summaries.len() < transfers.len() / 100 + 2);
    assert!(
        summaries
            .iter()
            .all(|s| s.elapsed >= 1000 && s.reports >= s.events)
    );
    let flushed = window.flush(transfers.len() as u64 * 5);
    let total: u32 = summaries
        .iter()
        .chain(flushed.iter())
        .map(|s| s.reports)
        .sum();
    assert_eq!(total as usize, expected);
    // An event whose length field disagrees with its reports is rejected.
    let mut bad = advertising_event(2, 5, 1);
    bad[3] = 3;
    assert!(advertisements(Event::parse(&bad).unwrap(), |_| {}).is_err());
}

#[test]
fn advertising_window_reports_only_address_tails_and_counts() {
    let mut w = Advertising::new();
    assert_eq!(w.due(5000), None);
    w.event(100);
    w.report(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    w.unsupported();
    assert_eq!(w.due(1099), None);
    let s = w.due(1100).unwrap();
    assert_eq!((s.events, s.reports, s.hid, s.unsupported), (1, 1, 0, 1));
    assert_eq!(s.last, Some([0x22, 0x11]));
    assert_eq!(w.due(9000), None);
    assert_eq!(stage::address_tail(&[1, 2, 3, 4, 5, 6]), [2, 1]);
}

#[test]
fn connection_complete_metadata_is_decoded_without_the_full_peer_address() {
    let mut p = [0u8; 19];
    p[0] = 1;
    p[1] = 0;
    p[2..4].copy_from_slice(&0x0040u16.to_le_bytes());
    p[4] = 0; // central
    p[5] = 1;
    p[6..12].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    p[12..14].copy_from_slice(&0x0018u16.to_le_bytes());
    p[14..16].copy_from_slice(&0u16.to_le_bytes());
    p[16..18].copy_from_slice(&0x01f4u16.to_le_bytes());
    let c = stage::connection_complete(&p).unwrap();
    assert_eq!(
        (c.status, c.handle, c.role, c.peer_type, c.peer_tail),
        (0, 0x40, 0, 1, [0xbb, 0xaa])
    );
    assert_eq!((c.interval, c.latency, c.timeout), (0x18, 0, 0x1f4));
    assert_eq!(stage::connection_complete(&p[..18]), None);
    p[0] = 2;
    assert_eq!(stage::connection_complete(&p), None);
}

#[test]
fn first_acl_exchange_carries_smp_pairing_pdus_with_credit_return() {
    let generation = 9;
    let handle = 0x0040;
    // CSR8510-class LE buffer: 27-byte ACL payload, a few packets.
    let mut credits = Credits::new(generation, handle, 2, 27).unwrap();
    // First TX: SMP Pairing Request (6 bytes) in one L2CAP frame on CID 6.
    let request = [0x01, 0x03, 0x00, 0x2d, 0x10, 0x00, 0x01];
    let mut tx = vec![];
    tx.extend_from_slice(&(request.len() as u16).to_le_bytes());
    tx.extend_from_slice(&6u16.to_le_bytes());
    tx.extend_from_slice(&request);
    credits.reserve(generation, tx.len()).unwrap();
    credits.reserve(generation, tx.len()).unwrap();
    assert_eq!(
        credits.reserve(generation, tx.len()),
        Err(musha_platform::bluetooth::Error::Busy)
    );
    // Number of Completed Packets returns both credits for this handle only.
    let completed = [0x13, 9, 2, 0x40, 0, 2, 0, 0x41, 0, 1, 0];
    credits
        .completed(generation, Event::parse(&completed).unwrap())
        .unwrap();
    credits.reserve(generation, tx.len()).unwrap();
    assert_eq!(stage::smp_name(tx[4]), "Pairing Request");

    // First RX: Pairing Response in one ACL packet (PB=10b first fragment).
    let mut r = Reassembly::new(generation, handle).unwrap();
    let response = [0x02, 0x03, 0x00, 0x2d, 0x10, 0x00, 0x01];
    let mut packet = vec![0x40, 0x20, (4 + response.len()) as u8, 0];
    packet.extend_from_slice(&(response.len() as u16).to_le_bytes());
    packet.extend_from_slice(&6u16.to_le_bytes());
    packet.extend_from_slice(&response);
    let mut got = vec![];
    assert_eq!(
        r.receive(generation, 10, &packet, |cid, sdu| got
            .push((cid, sdu.to_vec()))),
        Ok(true)
    );
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, 6);
    assert_eq!(stage::smp_name(got[0].1[0]), "Pairing Response");

    // Public Key (65-byte SDU) arrives fragmented over 27-byte ACL packets.
    let mut key = vec![0x0c];
    key.extend(std::iter::repeat_n(0x5a, 64));
    let mut l2cap = vec![];
    l2cap.extend_from_slice(&(key.len() as u16).to_le_bytes());
    l2cap.extend_from_slice(&6u16.to_le_bytes());
    l2cap.extend_from_slice(&key);
    let mut done = false;
    for (i, chunk) in l2cap.chunks(27).enumerate() {
        let mut p = vec![0x40, if i == 0 { 0x20 } else { 0x10 }, chunk.len() as u8, 0];
        p.extend_from_slice(chunk);
        done = r
            .receive(generation, 20 + i as u64, &p, |cid, sdu| {
                assert_eq!(cid, 6);
                assert_eq!(sdu.len(), 65);
                assert_eq!(stage::smp_name(sdu[0]), "Pairing Public Key");
            })
            .unwrap();
    }
    assert!(done);
    for (code, name) in [
        (0x03, "Pairing Confirm"),
        (0x04, "Pairing Random"),
        (0x05, "Pairing Failed"),
        (0x0d, "Pairing DHKey Check"),
        (0x0b, "Security Request"),
        (0x7f, "unknown"),
    ] {
        assert_eq!(stage::smp_name(code), name);
    }
}

#[test]
fn pairing_failed_codes_and_guidance_fit_the_viewer() {
    for (code, reason) in [
        (0x01, "BT PAIRING FAILED PASSKEY ENTRY"),
        (0x05, "BT PAIRING FAILED NOT SUPPORTED"),
        (0x08, "BT PAIRING FAILED UNSPECIFIED"),
        (0x09, "BT PAIRING FAILED REPEATED ATTEMPTS"),
        (0x0f, "BT PAIRING FAILED KEY REJECTED"),
        (0x10, "BT PAIRING FAILED OTHER"),
        (0xff, "BT PAIRING FAILED OTHER"),
    ] {
        assert_eq!(stage::pairing_failed(code), reason);
    }
    // Viewer rows hold 80 ASCII characters; its font has no other glyphs.
    for code in 0..=0x10 {
        let line = format!("Bluetooth unavailable: {}", stage::pairing_failed(code));
        assert!(line.len() <= 80 && line.is_ascii(), "{line}");
    }
    for text in [stage::PAIRING_NO_REPLY, stage::PAIRING_TIMED_OUT] {
        assert!(text.len() <= 80 && text.is_ascii(), "{text}");
        assert!(text.contains("pair button"));
    }
}

#[test]
fn own_random_static_address_and_create_connection_layout() {
    // Top bits forced to 11; all-zero or all-one random parts are redrawn.
    assert_eq!(
        stage::random_static_from([1, 2, 3, 4, 5, 6]),
        Some([1, 2, 3, 4, 5, 0xc6])
    );
    assert_eq!(stage::random_static_from([0; 6]), None);
    assert_eq!(stage::random_static_from([0xff; 6]), None);
    assert_eq!(
        stage::random_static_from([0xff, 0xff, 0xff, 0xff, 0xff, 0x3e]),
        Some([0xff, 0xff, 0xff, 0xff, 0xff, 0xfe])
    );
    let peer = [0x73, 0xdf, 3, 4, 5, 6];
    let random = stage::create_connection(0, &peer, 1);
    let public = stage::create_connection(0, &peer, 0);
    assert_eq!((random[5], &random[6..12], random[12]), (0, &peer[..], 1));
    assert_eq!(public[12], 0);
    assert_eq!(&random[..4], &[0x10, 0, 0x10, 0]);
    assert_eq!(&random[13..17], &[0x18, 0, 0x28, 0]);
    assert_eq!(&random[19..21], &[0xf4, 1]);
}
