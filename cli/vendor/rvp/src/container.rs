//! `.rvp` container framing (Package Format §1, §4):
//! `magic "RVP" | cver u8 | flags u8 | hlen u16 BE | [sig-block] | [enc-header] | core`,
//! sig-block = `slen u16 BE | minisig text`.

use crate::RvpError;

pub const MAGIC: &[u8; 3] = b"RVP";
pub const CVER: u8 = 1;
pub const FLAG_SIGNED: u8 = 0x01;
pub const FLAG_ENCRYPTED: u8 = 0x02;
const FLAG_RESERVED: u8 = !(FLAG_SIGNED | FLAG_ENCRYPTED);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub cver: u8,
    pub flags: u8,
    /// The minisign signature text from the sig-block (without the `slen` prefix).
    pub sig: Option<Vec<u8>>,
    pub core: Vec<u8>,
}

fn err(m: impl Into<String>) -> RvpError {
    RvpError::Container(m.into())
}

/// Wraps `core`; `sig` (minisign text) present => SIGNED flag + sig-block.
pub fn container_write(core: &[u8], sig: Option<&[u8]>) -> Result<Vec<u8>, RvpError> {
    let mut out = Vec::with_capacity(7 + core.len());
    out.extend_from_slice(MAGIC);
    out.push(CVER);
    match sig {
        None => {
            out.push(0);
            out.extend_from_slice(&0u16.to_be_bytes());
        }
        Some(s) => {
            if s.is_empty() {
                return Err(err("empty signature"));
            }
            let slen = u16::try_from(s.len()).map_err(|_| err("signature too long"))?;
            let hlen = slen
                .checked_add(2)
                .ok_or_else(|| err("signature too long"))?;
            out.push(FLAG_SIGNED);
            out.extend_from_slice(&hlen.to_be_bytes());
            out.extend_from_slice(&slen.to_be_bytes());
            out.extend_from_slice(s);
        }
    }
    out.extend_from_slice(core);
    Ok(out)
}

/// A container's framing without its core: what [`container_head`] reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub cver: u8,
    pub flags: u8,
    pub sig: Option<Vec<u8>>,
}

/// Reads a v1 container header from a stream, leaving `r` at the first core byte. Rejects bad
/// magic, `cver != 1`, ENCRYPTED (v2), reserved flag bits, and lengths that disagree.
pub fn container_head(r: &mut dyn std::io::Read) -> Result<Head, RvpError> {
    let mut hdr = [0u8; 7];
    r.read_exact(&mut hdr)
        .map_err(|_| err("shorter than the 7-byte header"))?;
    if &hdr[..3] != MAGIC {
        return Err(err("bad magic (not an .rvp file)"));
    }
    let (cver, flags) = (hdr[3], hdr[4]);
    if cver != CVER {
        return Err(err(format!(
            "container version {cver} is not supported by this v1 reader"
        )));
    }
    if flags & FLAG_ENCRYPTED != 0 {
        return Err(err("encrypted package needs a v2 daemon"));
    }
    if flags & FLAG_RESERVED != 0 {
        return Err(err(format!("reserved flag bits set: {flags:#04x}")));
    }
    let hlen = u16::from_be_bytes([hdr[5], hdr[6]]) as usize;
    let mut region = vec![0u8; hlen];
    r.read_exact(&mut region)
        .map_err(|_| err("hlen runs past the end of the file"))?;
    let sig = if flags & FLAG_SIGNED != 0 {
        let slen = region
            .get(..2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]) as usize)
            .ok_or_else(|| err("SIGNED but no sig-block"))?;
        if slen == 0 || 2 + slen != hlen {
            return Err(err("sig-block length does not match hlen"));
        }
        Some(region[2..].to_vec())
    } else {
        if hlen != 0 {
            return Err(err("unsigned container with non-empty header region"));
        }
        None
    };
    Ok(Head { cver, flags, sig })
}

/// Splits a v1 container held in memory ([`container_head`] + the rest as core).
pub fn container_read(buf: &[u8]) -> Result<Container, RvpError> {
    let mut rest = buf;
    let Head { cver, flags, sig } = container_head(&mut rest)?;
    Ok(Container {
        cver,
        flags,
        sig,
        core: rest.to_vec(),
    })
}
