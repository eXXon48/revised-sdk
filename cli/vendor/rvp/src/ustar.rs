//! Deterministic POSIX ustar (Package Format §3): sorted names, uid/gid/mtime 0,
//! mode 0644/0755, names ≤ 100 bytes (no prefix / GNU / pax), two zero blocks at the end.

use crate::RvpError;
use std::io::Read;

const BLOCK: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UstarEntry {
    pub name: String,
    pub data: Vec<u8>,
    pub is_dir: bool,
}

fn err(m: impl Into<String>) -> RvpError {
    RvpError::Ustar(m.into())
}

/// Octal, zero-padded to `width - 1` digits, NUL-terminated.
fn octal(field: &mut [u8], v: u64) -> Result<(), RvpError> {
    let s = format!("{:0w$o}", v, w = field.len() - 1);
    if s.len() != field.len() - 1 {
        return Err(err(format!(
            "value {v} does not fit a {}-byte field",
            field.len()
        )));
    }
    field[..s.len()].copy_from_slice(s.as_bytes());
    field[s.len()] = 0;
    Ok(())
}

fn checksum(h: &[u8]) -> u64 {
    h.iter()
        .enumerate()
        .map(|(i, &b)| {
            if (148..156).contains(&i) {
                32
            } else {
                b as u64
            }
        })
        .sum()
}

fn header(e: &UstarEntry) -> Result<[u8; BLOCK], RvpError> {
    if e.is_dir && !e.data.is_empty() {
        return Err(err(format!("directory entry has data: {}", e.name)));
    }
    header_for(&e.name, e.data.len() as u64, e.is_dir)
}

/// The exact header [`ustar_write`] emits for one member.
fn header_for(name_s: &str, size: u64, is_dir: bool) -> Result<[u8; BLOCK], RvpError> {
    let name = name_s.as_bytes();
    if name.is_empty() || name.len() > 100 {
        return Err(err(format!(
            "entry name is {} bytes, ustar allows 1..=100: {name_s}",
            name.len()
        )));
    }
    let mut h = [0u8; BLOCK];
    h[..name.len()].copy_from_slice(name);
    octal(&mut h[100..108], if is_dir { 0o755 } else { 0o644 })?;
    octal(&mut h[108..116], 0)?;
    octal(&mut h[116..124], 0)?;
    octal(&mut h[124..136], size)?;
    octal(&mut h[136..148], 0)?;
    h[156] = if is_dir { b'5' } else { b'0' };
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    let sum = checksum(&h);
    octal(&mut h[148..155], sum)?;
    h[155] = b' ';
    Ok(h)
}

/// Writes `entries` sorted by name (unsigned byte order) with fixed metadata.
pub fn ustar_write(entries: &[UstarEntry]) -> Result<Vec<u8>, RvpError> {
    let mut sorted: Vec<&UstarEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    if sorted.windows(2).any(|w| w[0].name == w[1].name) {
        return Err(err("duplicate entry name"));
    }
    let mut out = Vec::new();
    for e in sorted {
        out.extend_from_slice(&header(e)?);
        out.extend_from_slice(&e.data);
        out.resize(out.len().div_ceil(BLOCK) * BLOCK, 0);
    }
    out.resize(out.len() + 2 * BLOCK, 0);
    Ok(out)
}

fn parse_octal(field: &[u8]) -> Result<u64, RvpError> {
    let digits: Vec<u8> = field
        .iter()
        .copied()
        .take_while(|&b| b != 0 && b != b' ')
        .collect();
    if digits.is_empty() || digits.iter().any(|b| !(b'0'..=b'7').contains(b)) {
        return Err(err("bad octal field"));
    }
    let mut v: u64 = 0;
    for d in digits {
        v = v
            .checked_mul(8)
            .and_then(|v| v.checked_add((d - b'0') as u64))
            .ok_or_else(|| err("octal field overflow"))?;
    }
    Ok(v)
}

/// Reads a ustar archive written by [`ustar_write`]. Bounds-checks every field; never panics
/// on truncated or malformed input.
pub fn ustar_read(buf: &[u8]) -> Result<Vec<UstarEntry>, RvpError> {
    if !buf.len().is_multiple_of(BLOCK) {
        return Err(err("archive length is not a multiple of 512"));
    }
    let mut out = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    let mut off = 0;
    loop {
        let h = buf
            .get(off..off + BLOCK)
            .ok_or_else(|| err("missing end-of-archive blocks"))?;
        if h.iter().all(|&b| b == 0) {
            let end = buf
                .get(off + BLOCK..off + 2 * BLOCK)
                .ok_or_else(|| err("missing second end-of-archive block"))?;
            if end.iter().any(|&b| b != 0) || buf[off + 2 * BLOCK..].iter().any(|&b| b != 0) {
                return Err(err("data after end-of-archive"));
            }
            return Ok(out);
        }
        if &h[257..263] != b"ustar\0" || &h[263..265] != b"00" {
            return Err(err("not a POSIX ustar header"));
        }
        if parse_octal(&h[148..156])? != checksum(h) {
            return Err(err("header checksum mismatch"));
        }
        if h[345..500].iter().any(|&b| b != 0) {
            return Err(err("ustar prefix field is not supported"));
        }
        let name_len = h[..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&h[..name_len])
            .map_err(|_| err("entry name is not UTF-8"))?
            .to_string();
        let is_dir = match h[156] {
            b'0' | 0 => false,
            b'5' => true,
            t => {
                return Err(err(format!(
                    "unsupported entry type {:?} for {name}",
                    t as char
                )))
            }
        };
        let size =
            usize::try_from(parse_octal(&h[124..136])?).map_err(|_| err("entry size too large"))?;
        let start = off + BLOCK;
        let data = start
            .checked_add(size)
            .and_then(|end| buf.get(start..end))
            .ok_or_else(|| err(format!("entry data runs past end of archive: {name}")))?
            .to_vec();
        if !names.insert(name.clone()) {
            return Err(err(format!("duplicate entry name: {name}")));
        }
        out.push(UstarEntry { name, data, is_dir });
        off = start + size.div_ceil(BLOCK) * BLOCK;
    }
}

/// Member callback of [`ustar_stream`]: `(name, is_dir, data reader bounded to the member)`.
pub type OnMember<'a> = dyn FnMut(&str, bool, &mut dyn Read) -> Result<(), RvpError> + 'a;

fn fill(r: &mut dyn Read, b: &mut [u8]) -> Result<(), RvpError> {
    r.read_exact(b).map_err(|e| match e.kind() {
        std::io::ErrorKind::UnexpectedEof => err("truncated archive"),
        _ => err(format!("archive: {e}")),
    })
}

/// Streams a canonical ustar (exactly what [`ustar_write`] emits): every header must equal the
/// writer's, names strictly ascending, zero padding, two zero end blocks, then end of input.
/// `on` gets each member's data as a bounded reader; whatever it leaves unread is skipped, so
/// memory stays one block regardless of member size.
pub fn ustar_stream(r: &mut dyn Read, on: &mut OnMember) -> Result<(), RvpError> {
    let mut h = [0u8; BLOCK];
    let mut prev: Option<String> = None;
    loop {
        fill(r, &mut h)?;
        if h.iter().all(|&b| b == 0) {
            fill(r, &mut h)?;
            if h.iter().any(|&b| b != 0) {
                return Err(err("data after end-of-archive"));
            }
            let mut one = [0u8; 1];
            return match r.read(&mut one) {
                Ok(0) => Ok(()),
                Ok(_) => Err(err("data after end-of-archive")),
                Err(e) => Err(err(format!("archive: {e}"))),
            };
        }
        let name_len = h[..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&h[..name_len])
            .map_err(|_| err("entry name is not UTF-8"))?
            .to_string();
        let size = parse_octal(&h[124..136])?;
        let is_dir = h[156] == b'5';
        if let Some(p) = &prev {
            if *p == name {
                return Err(err(format!("duplicate entry name: {name}")));
            }
            if p.as_bytes() > name.as_bytes() {
                return Err(err("core is not canonical (member order)"));
            }
        }
        if header_for(&name, size, is_dir)? != h {
            return Err(err(format!(
                "core is not canonical (header of {name} differs from the packer)"
            )));
        }
        let mut data = (&mut *r).take(size);
        on(&name, is_dir, &mut data)?;
        std::io::copy(&mut data, &mut std::io::sink()).map_err(|e| err(format!("{name}: {e}")))?;
        if data.limit() != 0 {
            return Err(err(format!("entry data runs past end of archive: {name}")));
        }
        let pad = (BLOCK as u64 - size % BLOCK as u64) % BLOCK as u64;
        let mut p = [0u8; BLOCK];
        fill(r, &mut p[..pad as usize])?;
        if p.iter().any(|&b| b != 0) {
            return Err(err(format!("core is not canonical (padding after {name})")));
        }
        prev = Some(name);
    }
}
