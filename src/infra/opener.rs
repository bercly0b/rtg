//! Default browser opener using the `open` crate.

use anyhow::Result;

use super::contracts::ExternalOpener;

/// Opens URLs in the system's default browser via `open::that`.
#[derive(Debug, Clone, Default)]
pub struct BrowserOpener;

impl ExternalOpener for BrowserOpener {
    fn open(&self, target: &str) -> Result<()> {
        open::that(target)?;
        Ok(())
    }
}
