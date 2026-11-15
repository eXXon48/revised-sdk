//! Stock 74.00.324A rootfs path list (`stock/74.00.324A.paths`, one `<f|d|l> <abs path>` per
//! line, generated read-only from the stock rootfs image) and the override checks against it.

use std::collections::BTreeMap;

use crate::manifest::{FileType, Manifest};
use crate::RvpError;

/// Path -> node kind (`f` regular, `d` directory, `l` symlink).
pub struct Stock(BTreeMap<String, char>);

impl Stock {
    /// Malformed lines are skipped (the list is generated, not hand-written).
    pub fn parse(text: &str) -> Stock {
        Stock(
            text.lines()
                .filter_map(|l| l.split_once(' '))
                .filter_map(|(k, p)| Some((p.to_string(), k.chars().next()?)))
                .collect(),
        )
    }

    pub fn kind(&self, path: &str) -> Option<char> {
        self.0.get(path).copied()
    }
}

/// Overrides vs stock: `modify` edits an existing stock regular file; outward `new` and `link`
/// create a path stock does not have. Jailed (relative) entries are not checked.
// note: exact-path lookup only; a path reached through a stock symlinked parent dir is not
// resolved here — the daemon canonicalizes on the unit (Vanilla Backup Contract §3).
pub fn check_stock(m: &Manifest, stock: &Stock) -> Result<(), RvpError> {
    for f in m.files.iter().filter(|f| f.live.starts_with('/')) {
        let live = f.live.as_str();
        let bad = match (f.r#type, stock.kind(live)) {
            (FileType::Modify, Some('f')) => None,
            (FileType::Modify, None) => Some(format!("modify target is not a stock file: {live}")),
            (FileType::Modify, Some(k)) => Some(format!(
                "modify target must be a stock regular file (stock has `{k}`): {live}"
            )),
            (_, None) => None,
            (FileType::New, Some(_)) => Some(format!(
                "path exists in stock, use type modify to change it: {live}"
            )),
            (FileType::Link, Some(_)) => Some(format!("link path exists in stock: {live}")),
        };
        if let Some(msg) = bad {
            return Err(RvpError::Package(msg));
        }
    }
    Ok(())
}
