//! Opens URLs and files with the system default handler.

use std::path::Path;

use crate::extensions::{snippets, system};

pub struct SystemOpener;

impl snippets::OpenUrl for SystemOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        open::that_detached(url).map_err(|error| error.to_string())
    }
}

impl system::OpenFile for SystemOpener {
    fn open(&self, path: &Path) -> Result<(), String> {
        // A detached open returns once the handler starts, before it could
        // say the file is missing.
        path.metadata().map_err(|error| error.to_string())?;
        open::that_detached(path).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::system::OpenFile;

    #[test]
    fn a_missing_file_is_reported() {
        let missing = std::env::temp_dir().join("dango-no-such-file-7f3a");
        assert!(OpenFile::open(&SystemOpener, &missing).is_err());
    }
}
