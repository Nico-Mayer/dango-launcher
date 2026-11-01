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

/// `open -a` finds the app by name itself, and fails when there is none.
pub fn browser_target(app: &str) -> Option<String> {
    Some(app.to_string())
}
