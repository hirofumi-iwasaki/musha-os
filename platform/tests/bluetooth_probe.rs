use musha_platform::bluetooth::{Error, probe};
fn event(op: u16, result: &[u8]) -> Vec<u8> {
    let mut p = vec![14, (3 + result.len()) as u8, 1, op as u8, (op >> 8) as u8];
    p.extend_from_slice(result);
    p
}
#[test]
fn standard_only_and_exact_lengths() {
    for (op, n) in [
        (0x0c03, 1),
        (0x1001, 9),
        (0x1003, 9),
        (0x2003, 9),
        (0x1009, 7),
    ] {
        let bytes = event(op, &vec![0; n]);
        let r = probe::reply(op, &bytes).unwrap();
        assert_eq!(r.status, 0);
        assert_eq!(r.data.len(), n - 1);
        assert!(probe::reply(op, &event(op, &vec![0; n + 1])).is_err());
    }
    assert_eq!(
        probe::reply(0xfc05, &event(0xfc05, &[0])).unwrap_err(),
        Error::Unsupported
    );
    assert!(probe::COMMANDS.iter().all(|op| op >> 10 != 0x3f));
}
#[test]
fn status_failure_never_decodes_payload() {
    let p = event(0x2003, &[1]);
    let r = probe::reply(0x2003, &p).unwrap();
    assert_eq!(r.status, 1);
    assert!(r.data.is_empty());
    assert!(probe::reply(0x1001, &p).is_err());
}
#[test]
fn malformed_complete_and_credit_are_preserved() {
    assert!(probe::reply(0x0c03, &[14, 4, 1, 3, 12]).is_err());
    let mut p = event(0x0c03, &[0]);
    p[2] = 0;
    assert_eq!(probe::reply(0x0c03, &p).unwrap().credits, 0);
    assert!(probe::reply(0x0c03, &[15, 4, 0, 1, 3, 12]).is_err());
}
#[test]
fn version_fields_are_little_endian() {
    let p = event(0x1001, &[0, 6, 0x34, 0x12, 6, 2, 0, 0x78, 0x56]);
    let r = probe::reply(0x1001, &p).unwrap();
    assert_eq!(r.data[0], 6);
    assert_eq!(u16::from_le_bytes([r.data[1], r.data[2]]), 0x1234);
    assert_eq!(u16::from_le_bytes([r.data[4], r.data[5]]), 2);
}
