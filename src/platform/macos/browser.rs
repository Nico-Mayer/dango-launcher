use crate::browser::Browser;

/// Names `open -a` accepts.
pub fn browser_app(browser: Browser) -> Option<&'static str> {
    match browser {
        Browser::Chrome => Some("Google Chrome"),
        Browser::Edge => Some("Microsoft Edge"),
        Browser::Firefox => Some("Firefox"),
        Browser::Brave => Some("Brave Browser"),
        Browser::Safari => Some("Safari"),
    }
}
