use crate::browser::Browser;

/// Names under the registry's App Paths keys, which is where a browser's
/// installer says where its exe is.
pub fn browser_app(browser: Browser) -> Option<&'static str> {
    match browser {
        Browser::Chrome => Some("chrome.exe"),
        Browser::Edge => Some("msedge.exe"),
        Browser::Firefox => Some("firefox.exe"),
        Browser::Brave => Some("brave.exe"),
        Browser::Safari => None,
    }
}

/// The exe path App Paths gives for `app`, or `None` when it is not installed.
///
/// The `open` crate does this lookup itself, but keeps a trailing NUL of a
/// padded value (5.4.5), and `Command` then refuses the path. Given a full path,
/// it skips its own lookup.
pub fn browser_target(app: &str) -> Option<String> {
    let key = format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{app}");
    [
        windows_registry::CURRENT_USER,
        windows_registry::LOCAL_MACHINE,
    ]
    .into_iter()
    .find_map(|root| root.open(&key).ok()?.get_string("").ok())
    .filter(|path| !path.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safari_is_the_only_browser_missing_on_windows() {
        for browser in Browser::ALL {
            assert_eq!(browser_app(browser).is_none(), browser == Browser::Safari);
        }
    }

    #[test]
    fn an_app_that_is_not_installed_has_no_target() {
        assert_eq!(browser_target("dango-no-such-browser.exe"), None);
    }
}
