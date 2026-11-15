//! Debian-style relationship fields (`depends` / `conflicts`) and dpkg version comparison.
use crate::{pack::check_name, RvpError};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelOp {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rel {
    pub name: String,
    pub ver: Option<(RelOp, String)>,
}

fn err(m: impl Into<String>) -> RvpError {
    RvpError::Manifest(m.into())
}

fn parse_rel(s: &str) -> Result<Rel, RvpError> {
    let s = s.trim();
    let (name, ver) = match s.split_once('(') {
        None => (s, None),
        Some((n, rest)) => {
            let inner = rest
                .strip_suffix(')')
                .ok_or_else(|| err(format!("unterminated version in {s:?}")))?
                .trim();
            let (op, v) = [
                ("<<", RelOp::Lt),
                ("<=", RelOp::Le),
                (">>", RelOp::Gt),
                (">=", RelOp::Ge),
                ("=", RelOp::Eq),
            ]
            .into_iter()
            .find_map(|(t, op)| inner.strip_prefix(t).map(|v| (op, v.trim())))
            .ok_or_else(|| err(format!("bad version operator in {s:?}")))?;
            if v.is_empty()
                || !v
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".+~:-".contains(&b))
            {
                return Err(err(format!("bad version in {s:?}")));
            }
            (n.trim(), Some((op, v.to_string())))
        }
    };
    check_name("dependency name", name).map_err(|e| err(e.to_string()))?;
    Ok(Rel {
        name: name.into(),
        ver,
    })
}

/// `"a (>= 1), b | c"` -> `[[a>=1], [b, c]]`: outer AND (`,`), inner OR (`|`). Blank -> `[]`.
pub fn parse_rels(s: &str) -> Result<Vec<Vec<Rel>>, RvpError> {
    if s.trim().is_empty() {
        return Ok(vec![]);
    }
    s.split(',')
        .map(|c| c.split('|').map(parse_rel).collect())
        .collect()
}

fn order(c: Option<u8>) -> i32 {
    match c {
        Some(b'~') => -1,
        None => 0,
        Some(c) if c.is_ascii_digit() => 0,
        Some(c) if c.is_ascii_alphabetic() => c as i32,
        Some(c) => c as i32 + 256,
    }
}

/// dpkg `verrevcmp` on one upstream or revision part.
fn verrevcmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        while (i < a.len() && !a[i].is_ascii_digit()) || (j < b.len() && !b[j].is_ascii_digit()) {
            let d = order(a.get(i).copied()).cmp(&order(b.get(j).copied()));
            if d != Ordering::Equal {
                return d;
            }
            i += 1;
            j += 1;
        }
        let (da, ni) = run(a, i);
        let (db, nj) = run(b, j);
        (i, j) = (ni, nj);
        // digit runs compare numerically: strip leading zeros, then length, then bytes
        let (da, db) = (strip0(da), strip0(db));
        let d = da.len().cmp(&db.len()).then_with(|| da.cmp(db));
        if d != Ordering::Equal {
            return d;
        }
    }
    Ordering::Equal
}

fn run(s: &[u8], mut k: usize) -> (&[u8], usize) {
    let st = k;
    while k < s.len() && s[k].is_ascii_digit() {
        k += 1;
    }
    (&s[st..k], k)
}

fn strip0(d: &[u8]) -> &[u8] {
    let n = d.iter().take_while(|&&b| b == b'0').count();
    &d[n..]
}

fn split_ver(v: &str) -> (u64, &str, &str) {
    let (epoch, rest) = match v.split_once(':') {
        // note: unparsable epoch counts as 0; bounded-ness already guaranteed by parse_rel
        Some((e, r)) => (e.parse().unwrap_or(0), r),
        None => (0, v),
    };
    match rest.rsplit_once('-') {
        Some((u, r)) => (epoch, u, r),
        None => (epoch, rest, "0"),
    }
}

/// dpkg ordering: `epoch:upstream-revision`, `~` sorts before everything.
pub fn cmp_version(a: &str, b: &str) -> Ordering {
    let (ea, ua, ra) = split_ver(a);
    let (eb, ub, rb) = split_ver(b);
    ea.cmp(&eb)
        .then_with(|| verrevcmp(ua, ub))
        .then_with(|| verrevcmp(ra, rb))
}

/// Does installed `name`@`version` satisfy `rel`?
pub fn satisfies(rel: &Rel, name: &str, version: &str) -> bool {
    if rel.name != name {
        return false;
    }
    let Some((op, want)) = &rel.ver else {
        return true;
    };
    let o = cmp_version(version, want);
    match op {
        RelOp::Lt => o == Ordering::Less,
        RelOp::Le => o != Ordering::Greater,
        RelOp::Eq => o == Ordering::Equal,
        RelOp::Ge => o != Ordering::Less,
        RelOp::Gt => o == Ordering::Greater,
    }
}
