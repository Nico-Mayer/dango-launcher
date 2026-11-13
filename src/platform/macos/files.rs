use std::path::{Path, PathBuf};

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSArray, NSString, NSUserDefaults, NSURL};

/// Where the Screenshot app saves, as set in its Options menu. The value is
/// stored with a leading `~` when it is inside the home folder.
pub fn screenshot_folder() -> Option<PathBuf> {
    let domain = NSUserDefaults::standardUserDefaults()
        .persistentDomainForName(&NSString::from_str("com.apple.screencapture"));
    let location = domain
        .and_then(|domain| domain.objectForKey(&NSString::from_str("location")))
        .and_then(|value| value.downcast::<NSString>().ok())
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty());
    match location {
        Some(location) => Some(crate::platform::expand_home(&location)),
        None => dirs::desktop_dir(),
    }
}

pub fn reveal(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Err(std::io::Error::other("the file no longer exists"));
    }
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    NSWorkspace::sharedWorkspace()
        .activateFileViewerSelectingURLs(&NSArray::from_retained_slice(&[url]));
    Ok(())
}
