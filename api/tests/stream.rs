use musha_api::stream::{Entry, Error, Files, Format, MAX_FILE_BYTES, MAX_READ};
fn entry(bytes: u64) -> Entry {
    let mut name = [0; 255];
    name[..5].copy_from_slice(&[65, 46, 84, 88, 84]);
    Entry {
        name,
        name_len: 5,
        bytes,
    }
}
#[test]
fn both_formats_list_read_retry_eof_owned_result() {
    for format in [Format::Fat32, Format::Exfat] {
        let entries = [entry(5), entry(0)];
        let mut files = Files::new();
        files.mount(format, &entries).unwrap();
        assert_eq!(files.format(), Ok(format));
        let mut cursor = files.list_begin().unwrap();
        let (id, metadata) = files.list_next(&mut cursor).unwrap().unwrap();
        assert_eq!(metadata.bytes, 5);
        let empty = files.list_next(&mut cursor).unwrap().unwrap().0;
        assert!(files.list_next(&mut cursor).unwrap().is_none());
        let h = files.open_id(id).unwrap();
        let mut out = [0xa5; 3];
        assert_eq!(files.read(h, &mut out), Err(Error::Again));
        let req = files.take_request().unwrap();
        assert_eq!((req.index(), req.offset(), req.length()), (0, 0, 3));
        for _ in 0..100 {
            assert_eq!(files.read(h, &mut out), Err(Error::Again));
            assert!(files.take_request().is_none());
            assert_eq!(out, [0xa5; 3]);
        }
        let mut bytes = *b"abc";
        assert!(files.complete(req, Ok(&bytes)));
        bytes.fill(0);
        assert!(!files.complete(req, Ok(b"xxx")));
        assert_eq!(files.read(h, &mut [0; 2]), Err(Error::Invalid));
        assert_eq!(files.read(h, &mut out), Ok(3));
        assert_eq!(&out, b"abc");
        assert_eq!(files.read(h, &mut out), Err(Error::Again));
        let req = files.take_request().unwrap();
        assert_eq!(req.offset(), 3);
        assert!(files.complete(req, Ok(b"de")));
        assert_eq!(files.read(h, &mut out), Ok(2));
        assert_eq!(&out, b"dec");
        assert_eq!(files.read(h, &mut out), Ok(0));
        files.close(h).unwrap();
        assert_eq!(files.read(h, &mut out), Err(Error::Invalid));
        let h = files.open_id(empty).unwrap();
        assert_eq!(files.read(h, &mut out), Ok(0));
        assert!(files.take_request().is_none());
    }
}
#[test]
fn cancellation_quarantines_running_request_and_disconnect_invalidates_ids() {
    let entries = [entry(1)];
    let mut f = Files::new();
    f.mount(Format::Exfat, &entries).unwrap();
    let mut cursor = f.list_begin().unwrap();
    let id = f.list_next(&mut cursor).unwrap().unwrap().0;
    let h = f.open_id(id).unwrap();
    let mut out = [0xa5];
    assert_eq!(f.read(h, &mut out), Err(Error::Again));
    f.close(h).unwrap();
    assert!(f.take_request().is_none()); // queued cancellation
    let h = f.open_id(id).unwrap();
    assert_eq!(f.read(h, &mut out), Err(Error::Again));
    let req = f.take_request().unwrap();
    assert!(!f.request_canceled(req));
    f.close(h).unwrap();
    assert!(f.request_canceled(req));
    assert_eq!(f.open_id(id), Err(Error::Again));
    assert_eq!(f.mount(Format::Fat32, &entries), Err(Error::Again));
    f.disconnect();
    assert!(f.request_canceled(req));
    assert_eq!(f.read(h, &mut out), Err(Error::Disconnected));
    assert!(f.complete(req, Ok(b"a")));
    assert!(!f.request_canceled(req));
    assert_eq!(out, [0xa5]);
    f.mount(Format::Fat32, &entries).unwrap();
    assert_eq!(f.open_id(id), Err(Error::Invalid));
    assert!(matches!(f.list_next(&mut cursor), Err(Error::Invalid)));
    assert!(!f.complete(req, Ok(b"a")));
}
#[test]
fn terminal_errors_retry_new_token_and_short_reads_are_atomic() {
    let entries = [entry(3)];
    let mut f = Files::new();
    f.mount(Format::Fat32, &entries).unwrap();
    let id = f
        .list_next(&mut f.list_begin().unwrap())
        .unwrap()
        .unwrap()
        .0;
    let h = f.open_id(id).unwrap();
    let mut out = [0xa5; 3];
    for error in [Error::Io, Error::Corrupt, Error::Unsupported, Error::Limit] {
        assert_eq!(f.read(h, &mut out), Err(Error::Again));
        let old = f.take_request().unwrap();
        assert!(!f.complete(old, Err(Error::Again)));
        assert!(f.complete(old, Err(error)));
        assert_eq!(f.read(h, &mut out), Err(error));
        assert_eq!(out, [0xa5; 3]);
        assert_eq!(f.read(h, &mut out), Err(Error::Again));
        let new = f.take_request().unwrap();
        assert_eq!(new.offset(), 0);
        assert!(!f.complete(old, Ok(b"bad")));
        assert!(f.complete(new, Ok(b"a")));
        assert_eq!(f.read(h, &mut out), Err(Error::Corrupt));
        assert_eq!(out, [0xa5; 3]);
    }
}
#[test]
fn read_cap_empty_buffers_oversize_and_ready_cancellation() {
    let entries = [entry(MAX_FILE_BYTES + 1), entry(8192)];
    let mut f = Files::new();
    f.mount(Format::Exfat, &entries).unwrap();
    let mut cursor = f.list_begin().unwrap();
    assert_eq!(
        f.open_id(f.list_next(&mut cursor).unwrap().unwrap().0),
        Err(Error::TooLarge)
    );
    let id = f.list_next(&mut cursor).unwrap().unwrap().0;
    let h = f.open_id(id).unwrap();
    assert_eq!(f.read(h, &mut []), Ok(0));
    assert!(f.take_request().is_none());
    assert_eq!(f.read(h, &mut [0; 8192]), Err(Error::Again));
    let req = f.take_request().unwrap();
    assert_eq!(req.length(), MAX_READ);
    assert!(f.complete(req, Ok(&[42; MAX_READ])));
    f.close(h).unwrap();
    let h = f.open_id(id).unwrap();
    assert_eq!(f.read(h, &mut [0; 1]), Err(Error::Again));
    assert_eq!(f.take_request().unwrap().offset(), 0);
}
