//! Opens URLs and files with the system default handler.

use std::path::Path;

use crate::browser::Browser;
use crate::extensions::{snippets, system};

pub struct SystemOpener;

impl snippets::OpenUrl for SystemOpener {
    fn open(&self, url: &str) -> Result<(), String> {
        open::that_detached(url).map_err(|error| error.to_string())
    }

    fn open_in(&self, url: &str, browser: Option<&str>) -> Result<(), String> {
        if let Some(app) = app_for(browser) {
            let opened = match crate::platform::browser_target(app) {
                Some(target) => open::with_detached(url, target).map_err(|e| e.to_string()),
                None => Err("it is not installed".into()),
            };
            match opened {
                Ok(()) => return Ok(()),
                Err(error) => crate::log::append(&format!(
                    "{app} did not open {url}, using the default browser: {error}"
                )),
            }
        }
        self.open(url)
    }
}

fn app_for(browser: Option<&str>) -> Option<&'static str> {
    browser.and_then(Browser::from_id).and_then(Browser::app)
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

    #[test]
    fn an_unknown_browser_opens_in_the_default() {
        assert_eq!(app_for(Some("netscape")), None);
        assert_eq!(app_for(None), None);
    }

    #[test]
    fn a_browser_this_platform_lacks_opens_in_the_default() {
        for browser in Browser::ALL.into_iter().filter(|b| b.app().is_none()) {
            assert_eq!(app_for(Some(browser.id())), None);
        }
    }

    #[test]
    fn a_known_browser_opens_by_its_app_name() {
        assert_eq!(app_for(Some("firefox")), Browser::Firefox.app());
        assert!(app_for(Some("firefox")).is_some());
    }
}
