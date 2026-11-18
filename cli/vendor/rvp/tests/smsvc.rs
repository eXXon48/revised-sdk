mod common;
use common::fixture;
use rvp::manifest::Manifest;
use rvp::smsvc::{service_names, sm_services, SmService};
use rvp::ustar::UstarEntry;
use rvp::RvpError;

const SM: &str = "/jci/sm/sm.conf";
const WCP: &str = "/jci/sm/sm_WCP.conf";

fn svc(name: &str, autorun: &str) -> String {
    format!(
        "<service type=\"process\" name=\"{name}\" path=\"/x\" autorun=\"{autorun}\"></service>"
    )
}

fn payload(node: &str) -> Vec<u8> {
    let op = serde_json::json!({"ops":[{"insert":{"xpath":"//services","position":"after","node":node}}]});
    op.to_string().into_bytes()
}

/// (live, payload bytes) -> manifest + entries, each payload as `payload/<n>.json`.
fn pkg(files: &[(&str, Vec<u8>)]) -> (Manifest, Vec<UstarEntry>) {
    let fs: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, (live, _))| {
            serde_json::json!({"live": live, "payload": format!("payload/{i}.json"), "mode": "0755",
                "owner": "1000:1000", "type": "modify", "apply": "xml-node"})
        })
        .collect();
    let m = serde_json::json!({"formatVersion":1,"name":"t","version":"1.0.0","author":"a",
        "targetCMU":"74.00.324A","category":"System mods","description":"d","minDaemon":"0.1.0",
        "depends":"","conflicts":"","restart":false,"files":fs});
    let e = files
        .iter()
        .enumerate()
        .map(|(i, (_, d))| UstarEntry { name: format!("payload/{i}.json"), data: d.clone(), is_dir: false })
        .collect();
    (Manifest::parse(m.to_string().as_bytes()).unwrap(), e)
}

fn both(sm: Vec<u8>, wcp: Vec<u8>) -> Result<Vec<SmService>, RvpError> {
    let (m, e) = pkg(&[(SM, sm), (WCP, wcp)]);
    sm_services(&m, &e)
}

#[test]
fn aa_touchbridged_payload_gives_one_always_on_service() {
    let rd = |f| std::fs::read(fixture(&format!("smsvc/{f}"))).unwrap();
    let r = both(rd("sm.json"), rd("sm_WCP.json")).unwrap();
    assert_eq!(r, vec![SmService { name: "aa_touchbridged".into(), on_demand: false }]);
}

#[test]
fn autorun_no_is_on_demand() {
    let p = payload(&svc("a", "no"));
    let r = both(p.clone(), p).unwrap();
    assert_eq!(r, vec![SmService { name: "a".into(), on_demand: true }]);
}

#[test]
fn two_services_in_one_node_refused() {
    let p = payload(&format!("{}{}", svc("a", "yes"), svc("b", "yes")));
    assert!(matches!(both(p.clone(), p), Err(RvpError::Package(_))));
}

#[test]
fn non_service_node_refused() {
    let p = payload("<other name=\"a\" autorun=\"yes\"/>");
    assert!(matches!(both(p.clone(), p), Err(RvpError::Package(_))));
}

#[test]
fn sm_and_wcp_disagree_refused() {
    let r = both(payload(&svc("a", "yes")), payload(&svc("a", "no")));
    assert!(matches!(r, Err(RvpError::Package(_))));
}

#[test]
fn missing_wcp_counterpart_refused() {
    let (m, e) = pkg(&[(SM, payload(&svc("a", "yes")))]);
    assert!(matches!(sm_services(&m, &e), Err(RvpError::Package(_))));
}

#[test]
fn missing_autorun_refused() {
    let p = payload("<service name=\"a\"/>");
    assert!(matches!(both(p.clone(), p), Err(RvpError::Package(_))));
}

#[test]
fn package_without_sm_files_gives_empty() {
    let (m, e) = pkg(&[("/etc/foo.xml", payload("<a/>"))]);
    assert_eq!(sm_services(&m, &e).unwrap(), vec![]);
}

#[test]
fn service_name_outside_charset_refused() {
    for bad in ["-x", "a b", "a;b", "a-b", ""] {
        let p = payload(&svc(bad, "no"));
        assert!(matches!(both(p.clone(), p), Err(RvpError::Package(_))), "{bad:?}");
    }
    let p = payload(&svc("Ok_9", "no"));
    assert!(both(p.clone(), p).is_ok());
}

#[test]
fn service_names_lists_every_service() {
    let doc = format!("<sm><services>{}{}</services><x name=\"no\"/></sm>", svc("a", "yes"), svc("b", "no"));
    assert_eq!(service_names(&doc).unwrap().into_iter().collect::<Vec<_>>(), ["a", "b"]);
}
