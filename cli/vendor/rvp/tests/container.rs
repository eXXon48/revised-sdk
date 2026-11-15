use rvp::container::{container_read, container_write, FLAG_ENCRYPTED, FLAG_SIGNED};

const CORE: &[u8] = b"core-bytes";
const SIG: &[u8] = b"untrusted comment: x\nAAAA\ntrusted comment: y\nBBBB\n";

#[test]
fn test_container_roundtrip() {
    let rvp = container_write(CORE, Some(SIG)).unwrap();
    assert_eq!(&rvp[..4], b"RVP\x01");
    let c = container_read(&rvp).unwrap();
    assert_eq!(c.core, CORE);
    assert_eq!(c.sig.as_deref(), Some(SIG));
    assert_eq!(c.flags & FLAG_SIGNED, FLAG_SIGNED);

    let rvp = container_write(CORE, None).unwrap();
    assert_eq!(&rvp[..7], b"RVP\x01\x00\x00\x00");
    let c = container_read(&rvp).unwrap();
    assert_eq!(c.flags & FLAG_SIGNED, 0);
    assert_eq!(c.sig, None);
    assert_eq!(c.core, CORE);
}

#[test]
fn test_reject_encrypted_flag() {
    let mut rvp = container_write(CORE, None).unwrap();
    rvp[4] |= FLAG_ENCRYPTED;
    let e = container_read(&rvp).unwrap_err().to_string();
    assert!(e.contains("v2"), "{e}");
}

#[test]
fn test_reject_bad_magic_cver() {
    let good = container_write(CORE, Some(SIG)).unwrap();
    let mut bad = good.clone();
    bad[0] = b'X';
    assert!(container_read(&bad).is_err());
    let mut bad = good.clone();
    bad[3] = 2;
    assert!(container_read(&bad).is_err());
    // hlen overruns the buffer
    let mut bad = good.clone();
    bad[5] = 0xff;
    bad[6] = 0xff;
    assert!(container_read(&bad).is_err());
    // slen inside the sig-block overruns hlen
    let mut bad = good.clone();
    bad[7] = 0xff;
    assert!(container_read(&bad).is_err());
    // reserved flag bit
    let mut bad = good.clone();
    bad[4] |= 0x04;
    assert!(container_read(&bad).is_err());
    // SIGNED with hlen 0 / unsigned with hlen > 0
    let mut bad = container_write(CORE, None).unwrap();
    bad[4] = FLAG_SIGNED;
    assert!(container_read(&bad).is_err());
    let mut bad = good.clone();
    bad[4] = 0;
    assert!(container_read(&bad).is_err());
    // short buffers never panic
    for n in 0..8 {
        assert!(container_read(&good[..n]).is_err());
    }
}
