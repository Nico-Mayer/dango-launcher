use crate::browser::Browser;

/// Names under the registry's App Paths keys, which is where the `open` crate
/// looks an app name up before starting it.
pub fn browser_app(browser: Browser) -> Option<&'static str> {
    match browser {
        Browser::Chrome => Some("chrome.exe"),
        Browser::Edge => Some("msedge.exe"),
        Browser::Firefox => Some("firefox.exe"),
        Browser::Brave => Some("brave.exe"),
        Browser::Safari => None,
    }
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
}
