use rvp::manifest::{FileType, Manifest};

const OK: &[u8] = include_bytes!("fixtures/manifest.ok.json");
const BAD_ENUM: &[u8] = include_bytes!("fixtures/manifest.badenum.json");

fn ok_str() -> String {
    String::from_utf8(OK.to_vec()).unwrap()
}

#[test]
fn test_parse_ok() {
    let m = Manifest::parse(OK).unwrap();
    assert_eq!(m.files.len(), 1);
    assert_eq!(m.files[0].r#type, FileType::Modify);
    assert_eq!(m.files[0].owner, "1000:1000");
    assert_eq!(m.format_version, 1);
    assert_eq!(m.target_cmu, "74.00.324A");
    assert_eq!(m.depends, "t-basic (>= 1.0)");
}

#[test]
fn test_roundtrip() {
    let m = Manifest::parse(OK).unwrap();
    let canon = m.to_canonical_json().unwrap();
    let m2 = Manifest::parse(&canon).unwrap();
    assert_eq!(m, m2);
    // canonical is a fixed point and sorts the opaque revert keys
    assert_eq!(m2.to_canonical_json().unwrap(), canon);
    let s = String::from_utf8(canon).unwrap();
    assert!(s.find("\"a\":").unwrap() < s.find("\"z\":").unwrap());
}

#[test]
fn test_reject_bad_enum() {
    let e = Manifest::parse(BAD_ENUM).unwrap_err().to_string();
    assert!(e.contains("apply"), "{e}");
    let xmlpatch = ok_str().replace("\"modify\"", "\"xmlpatch\"");
    let e = Manifest::parse(xmlpatch.as_bytes())
        .unwrap_err()
        .to_string();
    assert!(e.contains("type"), "{e}");
}

#[test]
fn test_ignore_unknown_field() {
    let s = ok_str().replacen('{', "{ \"future\": true,", 1);
    assert!(Manifest::parse(s.as_bytes()).is_ok());
}

#[test]
fn test_reject_missing_required() {
    let s = ok_str().replace("\"name\": \"bootspeed\",", "");
    let e = Manifest::parse(s.as_bytes()).unwrap_err().to_string();
    assert!(e.contains("name"), "{e}");
}

#[test]
fn manifest_restart_roundtrip() {
    let m = Manifest::parse(OK).unwrap();
    assert!(!m.restart);
    // absent field: canonical bytes carry no `restart`
    let canon = m.to_canonical_json().unwrap();
    assert!(!String::from_utf8(canon.clone()).unwrap().contains("restart"));
    assert_eq!(Manifest::parse(&canon).unwrap().to_canonical_json().unwrap(), canon);
    let mut v: serde_json::Value = serde_json::from_slice(OK).unwrap();
    v["restart"] = true.into();
    let m = Manifest::parse(v.to_string().as_bytes()).unwrap();
    assert!(m.restart);
    let canon = m.to_canonical_json().unwrap();
    assert!(String::from_utf8(canon.clone()).unwrap().contains("\"restart\":true"));
    assert_eq!(Manifest::parse(&canon).unwrap(), m);
}
