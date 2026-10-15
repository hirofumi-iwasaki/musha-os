// Copyright 2026 Hirofumi Iwasaki
// SPDX-License-Identifier: Apache-2.0
use musha_platform::bluetooth::{Error, stream::Stream};
#[test]
fn every_event_split_and_coalesced_packets() {
    let bytes = [0x0e, 4, 1, 3, 12, 0, 0x0f, 4, 0, 1, 3, 12];
    for split in 0..=bytes.len() {
        let mut s = Stream::<257>::new(7, false);
        let mut got = Vec::new();
        for part in [&bytes[..split], &bytes[split..]] {
            let mut input = part;
            while !input.is_empty() {
                if let Some(p) = s.next(7, 10, &mut input).unwrap() {
                    got.push(p.to_vec());
                }
            }
        }
        assert_eq!(got, vec![bytes[..6].to_vec(), bytes[6..].to_vec()]);
    }
}
#[test]
fn acl_fragmented_header_payload_and_shared_buffer_reuse() {
    let mut s = Stream::<1028>::new(3, true);
    let mut dma = [1, 0x20, 3, 0, 10, 20, 30, 2, 0x20, 0, 0];
    let mut input = &dma[..5];
    assert!(s.next(3, 1, &mut input).unwrap().is_none());
    dma[..5].fill(0xff); // earlier bytes were copied, not borrowed.
    let mut input = &dma[5..];
    assert_eq!(
        s.next(3, 2, &mut input).unwrap().unwrap(),
        [1, 0x20, 3, 0, 10, 20, 30]
    );
    assert_eq!(s.next(3, 2, &mut input).unwrap().unwrap(), [2, 0x20, 0, 0]);
}
#[test]
fn malformed_timeout_stale_and_clock_reversal_poison() {
    let mut s = Stream::<8>::new(1, true);
    let mut b = &[1, 0, 0xff, 0xff][..];
    assert_eq!(s.next(1, 0, &mut b), Err(Error::Length));
    assert_eq!(s.check(1, 1), Err(Error::Stale));
    let mut s = Stream::<257>::new(1, false);
    let mut b = &[0x0e][..];
    s.next(1, 4, &mut b).unwrap();
    assert_eq!(s.check(1, 1004), Err(Error::Timeout));
    let mut s = Stream::<257>::new(1, false);
    let mut b = &[0x0e][..];
    s.next(1, 4, &mut b).unwrap();
    assert_eq!(s.check(2, 5), Err(Error::Stale));
    let mut s = Stream::<257>::new(1, false);
    s.check(1, 4).unwrap();
    assert_eq!(s.check(1, 3), Err(Error::Timeout));
}
#[test]
fn exact_capacity_and_empty_completions_are_bounded() {
    let mut s = Stream::<257>::new(1, false);
    let mut packet = [0; 257];
    packet[1] = 255;
    let mut b = &packet[..];
    assert_eq!(s.next(1, 0, &mut b).unwrap().unwrap().len(), 257);
    let mut b = &[][..];
    assert!(s.next(1, 5000, &mut b).unwrap().is_none());
    assert!(core::mem::size_of::<Stream<1028>>() <= 1100);
}
#[test]
fn fault_and_generation_restart_always_begin_with_hci_reset() {
    use musha_platform::bluetooth::{
        Event,
        lifecycle::{Loss, loss},
        scan::Scan,
    };
    assert_eq!(
        loss(Event::parse(&[0x10, 1, 7]).unwrap(), 4),
        Ok(Some(Loss::Hardware))
    );
    assert!(loss(Event::parse(&[0x10, 0]).unwrap(), 4).is_err());
    assert_eq!(
        loss(Event::parse(&[5, 4, 0, 4, 0, 0x13]).unwrap(), 4),
        Ok(Some(Loss::Disconnected))
    );
    assert!(loss(Event::parse(&[5, 4, 0, 5, 0, 0x13]).unwrap(), 4).is_err());
    for generation in [1, 2, 3] {
        let mut scan = Scan::new(generation);
        let c = scan.next(generation, 0).unwrap().unwrap();
        assert_eq!(&c.bytes[..c.length], &[3, 12, 0]);
        assert_eq!(scan.next(generation, 1), Err(Error::Busy));
    }
}

#[test]
fn le_shared_capacity_fallback_and_credit_budget() {
    use musha_platform::bluetooth::acl::{Credits, capacity};
    assert_eq!(capacity(0x2002, &[0, 0, 0, 0]), Ok(None));
    assert_eq!(capacity(0x2002, &[0, 27, 0, 2]), Ok(Some((27, 2))));
    assert_eq!(
        capacity(0x1005, &[0, 0xff, 3, 0, 2, 0, 0, 0]),
        Ok(Some((1023, 2)))
    );
    for p in [
        &[0, 0, 0, 0, 0, 0, 0, 0][..],
        &[0, 0, 8, 0, 2, 0, 0, 0],
        &[1, 27, 0, 0, 2, 0, 0, 0],
    ] {
        assert!(capacity(0x1005, p).is_err());
    }
    assert_eq!(capacity(0x1005, &[0, 27]), Err(Error::Length));
    let mut c = Credits::new(3, 4, 2, 1023).unwrap();
    c.reserve(3, 27).unwrap();
    c.reserve(3, 27).unwrap();
    assert!(c.reserve(3, 27).is_err());
    assert!(c.reserve(4, 27).is_err());
}
#[test]
fn maximum_acl_every_usb_boundary_and_independent_event_stream() {
    let mut packet=[0u8;1028];packet[..4].copy_from_slice(&[1,0x20,0,4]);
    for split in 0..=packet.len(){
        let mut acl=Stream::<1028>::new(1,true);let mut event=Stream::<257>::new(1,false);
        let mut first=&packet[..split];let mut count=0;
        if let Some(p)=acl.next(1,0,&mut first).unwrap(){assert_eq!(p,packet);count+=1;}
        let mut e=&[0x10,1,7][..];assert_eq!(event.next(1,1,&mut e).unwrap().unwrap(),[0x10,1,7]);
        let mut rest=&packet[split..];if let Some(p)=acl.next(1,2,&mut rest).unwrap(){assert_eq!(p,packet);count+=1;}
        assert_eq!(count,1);assert!(rest.is_empty());
    }
}
