use rvp::ustar::{ustar_read, ustar_write, UstarEntry};

fn file(name: &str, data: &[u8]) -> UstarEntry {
    UstarEntry {
        name: name.into(),
        data: data.to_vec(),
        is_dir: false,
    }
}

#[test]
fn test_write_deterministic() {
    let a = vec![file("manifest.json", b"{}"), file("payload/x", b"hello")];
    let b = vec![file("payload/x", b"hello"), file("manifest.json", b"{}")];
    let wa = ustar_write(&a).unwrap();
    let wb = ustar_write(&b).unwrap();
    assert_eq!(wa, wb);
    assert_eq!(wa.len() % 512, 0);
    // POSIX ustar magic + version, uid/gid/mtime zero, mode normalized
    assert_eq!(&wa[257..265], b"ustar\x0000");
    assert_eq!(&wa[100..108], b"0000644\0");
    assert_eq!(&wa[108..116], b"0000000\0");
    assert_eq!(&wa[136..148], b"00000000000\0");
}

#[test]
fn test_roundtrip() {
    let entries = vec![
        file("manifest.json", b"{}"),
        file("payload/a/b.sh", &[7u8; 1000]),
        file("payload/empty", b""),
    ];
    let back = ustar_read(&ustar_write(&entries).unwrap()).unwrap();
    assert_eq!(back, entries);
}

#[test]
fn test_reject_long_name() {
    let e = ustar_write(&[file(&"a".repeat(120), b"x")])
        .unwrap_err()
        .to_string();
    assert!(e.contains("100"), "{e}");
}

#[test]
fn test_read_rejects_truncated_and_bad_checksum() {
    let w = ustar_write(&[file("payload/x", &[1u8; 600])]).unwrap();
    // data runs past the end
    assert!(ustar_read(&w[..700]).is_err());
    // header byte flipped -> checksum mismatch
    let mut bad = w.clone();
    bad[0] ^= 1;
    assert!(ustar_read(&bad).is_err());
    // not a multiple of 512 / garbage
    assert!(ustar_read(&[0u8; 100]).is_err());
}
