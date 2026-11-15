use std::fmt;

/// The single error type of the crate. Each variant carries the context callers report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RvpError {
    /// `manifest.json` failed to parse or validate; message names the field/value.
    Manifest(String),
    /// The ustar core is malformed or an entry cannot be archived.
    Ustar(String),
    /// Package tree / core content is invalid (missing, orphan, unsafe path...).
    Package(String),
    /// The `.rvp` container framing is invalid or needs a newer reader.
    Container(String),
    /// Signature or public key invalid / not acceptable.
    Verify(String),
}

impl fmt::Display for RvpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RvpError::Manifest(m) => write!(f, "manifest: {m}"),
            RvpError::Ustar(m) => write!(f, "ustar: {m}"),
            RvpError::Package(m) => write!(f, "package: {m}"),
            RvpError::Container(m) => write!(f, "container: {m}"),
            RvpError::Verify(m) => write!(f, "verify: {m}"),
        }
    }
}

impl std::error::Error for RvpError {}
