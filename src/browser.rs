//! The browsers a quicklink can name. The id is what `quicklinks.toml` stores,
//! the same on both platforms, so one file works on either machine.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Edge,
    Firefox,
    Brave,
    Safari,
}

impl Browser {
    /// The form's id for "Default browser", which stores no browser.
    pub const DEFAULT_ID: &str = "default";

    pub const ALL: [Browser; 5] = [
        Browser::Chrome,
        Browser::Edge,
        Browser::Firefox,
        Browser::Brave,
        Browser::Safari,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Browser::Chrome => "chrome",
            Browser::Edge => "edge",
            Browser::Firefox => "firefox",
            Browser::Brave => "brave",
            Browser::Safari => "safari",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Browser::Chrome => "Google Chrome",
            Browser::Edge => "Microsoft Edge",
            Browser::Firefox => "Firefox",
            Browser::Brave => "Brave",
            Browser::Safari => "Safari",
        }
    }

    pub fn from_id(id: &str) -> Option<Browser> {
        Browser::ALL.into_iter().find(|browser| browser.id() == id)
    }

    /// The name the platform starts this browser by, or `None` where it does
    /// not exist.
    pub fn app(self) -> Option<&'static str> {
        crate::platform::browser_app(self)
    }

    /// The browsers this platform can open, in the order the form lists them.
    pub fn offered() -> impl Iterator<Item = Browser> {
        Browser::ALL
            .into_iter()
            .filter(|browser| browser.app().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_round_trips() {
        for browser in Browser::ALL {
            assert_eq!(Browser::from_id(browser.id()), Some(browser));
        }
    }

    #[test]
    fn an_unknown_id_is_no_browser() {
        assert_eq!(Browser::from_id("netscape"), None);
        assert_eq!(Browser::from_id(""), None);
    }
}
