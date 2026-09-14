//! The Windows application indexer: enumerates `shell:AppsFolder` so packaged
//! and conventional apps appear together, extracts icons, and launches by the
//! shell identifier, bringing an already-running window forward first.

use std::path::Path;

use windows::core::{Interface, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, SIZE, TRUE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::Storage::EnhancedStorage::{PKEY_AppUserModel_ID, PKEY_Link_TargetParsingPath};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, IShellItem2, IShellItemImageFactory,
    SHCreateItemFromParsingName, ShellExecuteW, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
    SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetForegroundWindow, GetWindow,
    GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, GW_OWNER,
    SW_RESTORE, SW_SHOWNORMAL,
};

use crate::extensions::applications::{AppIndexer, IconRgba, IndexedApp, LaunchError};

pub struct WindowsAppIndexer;

impl AppIndexer for WindowsAppIndexer {
    fn index(&self) -> Vec<IndexedApp> {
        unsafe {
            init_com();
            enumerate().unwrap_or_default()
        }
    }

    fn launch(&self, app: &IndexedApp) -> Result<(), LaunchError> {
        // A filesystem target that has vanished is a stale entry, reported so
        // the caller can drop it.
        if let Some(target) = &app.target {
            if is_file_path(target) && !Path::new(target).exists() {
                return Err(LaunchError::NotFound);
            }
        }
        unsafe {
            init_com();
            if let Some(hwnd) = find_running_window(app) {
                bring_to_foreground(hwnd);
                return Ok(());
            }
            let file = HSTRING::from(apps_folder_path(&app.id));
            let instance = ShellExecuteW(
                None,
                PCWSTR::null(),
                &file,
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            // ShellExecute signals success with a value above 32.
            if instance.0 as isize > 32 {
                Ok(())
            } else {
                Err(LaunchError::Failed(format!(
                    "Couldn't open {}. Try again.",
                    app.name
                )))
            }
        }
    }

    /// The path is quoted on its own. Quoting the whole `/select,` argument, as
    /// `arg` does for anything with a space, makes Explorer ignore it and open
    /// the default folder instead.
    fn reveal(&self, app: &IndexedApp) -> std::io::Result<()> {
        use std::os::windows::process::CommandExt;
        match &app.target {
            Some(target) if is_file_path(target) => std::process::Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{target}\""))
                .spawn()
                .map(|_| ()),
            _ => Err(std::io::Error::other(
                "this application has no file to reveal",
            )),
        }
    }

    fn icon(&self, app: &IndexedApp, size: u32) -> Option<IconRgba> {
        icon_for(&apps_folder_path(&app.id), size)
    }
}

/// Renders the icon the shell shows for a parsing name, which is an
/// `shell:AppsFolder` entry for an indexed application and an executable path
/// for a running one. Shared so neither extension depends on the other.
pub fn icon_for(parsing_name: &str, size: u32) -> Option<IconRgba> {
    unsafe {
        init_com();
        let item: IShellItem =
            SHCreateItemFromParsingName(PCWSTR(HSTRING::from(parsing_name).as_ptr()), None).ok()?;
        let factory: IShellItemImageFactory = item.cast().ok()?;
        let bitmap = factory
            .GetImage(
                SIZE {
                    cx: size as i32,
                    cy: size as i32,
                },
                SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK,
            )
            .ok()?;
        let rgba = bitmap_to_rgba(bitmap);
        let _ = DeleteObject(bitmap.into());
        rgba
    }
}

/// Ignores the result: a second apartment-threaded init on the same thread
/// returns S_FALSE, and a background thread has no prior state to disturb.
pub(super) unsafe fn init_com() {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
}

unsafe fn enumerate() -> windows::core::Result<Vec<IndexedApp>> {
    let folder: IShellItem =
        SHCreateItemFromParsingName(PCWSTR(HSTRING::from("shell:AppsFolder").as_ptr()), None)?;
    let items: IEnumShellItems = folder.BindToHandler(None, &BHID_EnumItems)?;
    let mut apps = Vec::new();
    loop {
        let mut slot = [None];
        let mut fetched = 0;
        items.Next(&mut slot, Some(&mut fetched))?;
        let Some(item) = slot[0].take().filter(|_| fetched > 0) else {
            break;
        };
        let Ok(id) = item
            .GetDisplayName(SIGDN_PARENTRELATIVEPARSING)
            .map(take_pwstr)
        else {
            continue;
        };
        let name = item
            .GetDisplayName(SIGDN_NORMALDISPLAY)
            .map(take_pwstr)
            .unwrap_or_else(|_| id.clone());
        let target = item
            .cast::<IShellItem2>()
            .ok()
            .and_then(|item2| item2.GetString(&PKEY_Link_TargetParsingPath).ok())
            .map(take_pwstr);

        if !is_launchable(&id, target.as_deref()) {
            continue;
        }
        apps.push(IndexedApp { id, name, target });
    }
    Ok(apps)
}

/// Keeps real applications and drops the shell folder's non-apps: web links,
/// documents, and uninstall or maintenance shortcuts. Judged by the target
/// rather than by name, since names are localised.
fn is_launchable(id: &str, target: Option<&str>) -> bool {
    if id.contains('!') {
        return true; // packaged app
    }
    let Some(target) = target else {
        return !id.is_empty();
    };
    let lower = target.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return false;
    }
    let base = lower.rsplit(['\\', '/']).next().unwrap_or(&lower);
    if base == "msiexec.exe" || (base.starts_with("unins") && base.ends_with(".exe")) {
        return false;
    }
    if lower.ends_with(".exe") || lower.ends_with(".msc") {
        return true;
    }
    if target.starts_with("::{") {
        return true; // shell namespace object
    }
    // A custom URL scheme such as steam:// is launchable; a plain document is not.
    matches!(lower.find("://"), Some(pos) if pos > 0)
}

struct Collector {
    hwnds: Vec<HWND>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let collector = &mut *(lparam.0 as *mut Collector);
    let owner = GetWindow(hwnd, GW_OWNER).unwrap_or_default();
    if IsWindowVisible(hwnd).as_bool() && owner.0.is_null() {
        collector.hwnds.push(hwnd);
    }
    TRUE
}

/// Finds a top-level window belonging to the application: by AppUserModelID for
/// a packaged app, or by matching the owning process image path to the target
/// executable for a conventional one.
unsafe fn find_running_window(app: &IndexedApp) -> Option<HWND> {
    let mut collector = Collector { hwnds: Vec::new() };
    let _ = EnumWindows(Some(collect), LPARAM(&mut collector as *mut _ as isize));

    let packaged = app.id.contains('!');
    let target = app
        .target
        .as_deref()
        .map(str::to_lowercase)
        .filter(|t| t.ends_with(".exe"));

    for hwnd in collector.hwnds {
        if packaged {
            if window_aumid(hwnd).is_some_and(|a| a.eq_ignore_ascii_case(&app.id)) {
                return Some(hwnd);
            }
        } else if let Some(target) = &target {
            if window_process_path(hwnd).is_some_and(|p| p.to_lowercase() == *target) {
                return Some(hwnd);
            }
        }
    }
    None
}

pub(super) unsafe fn window_aumid(hwnd: HWND) -> Option<String> {
    let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd).ok()?;
    let mut value = store.GetValue(&PKEY_AppUserModel_ID).ok()?;
    let text = PropVariantToStringAlloc(&value).ok().map(take_pwstr);
    let _ = PropVariantClear(&mut value);
    text.filter(|s| !s.is_empty())
}

unsafe fn window_process_path(hwnd: HWND) -> Option<String> {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return None;
    }
    process_image_path(pid)
}

pub(super) unsafe fn process_image_path(pid: u32) -> Option<String> {
    let process: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = [0u16; 512];
    let mut len = buffer.len() as u32;
    let result = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_WIN32,
        PWSTR(buffer.as_mut_ptr()),
        &mut len,
    );
    let _ = CloseHandle(process);
    result.ok()?;
    Some(String::from_utf16_lossy(&buffer[..len as usize]))
}

/// Windows will not let a background process raise a window it does not own, so
/// it borrows the foreground thread's input queue for the call, the same lift
/// used by the launcher window itself.
unsafe fn bring_to_foreground(hwnd: HWND) {
    let _ = ShowWindow(hwnd, SW_RESTORE);
    let foreground = GetForegroundWindow();
    if foreground.0.is_null() {
        let _ = SetForegroundWindow(hwnd);
        return;
    }
    let current = GetCurrentThreadId();
    let owner = GetWindowThreadProcessId(foreground, None);
    let _ = AttachThreadInput(current, owner, true);
    let _ = AllowSetForegroundWindow(u32::MAX);
    let _ = SetForegroundWindow(hwnd);
    let _ = AttachThreadInput(current, owner, false);
    let _ = TRUE;
}

unsafe fn bitmap_to_rgba(hbitmap: HBITMAP) -> Option<IconRgba> {
    let mut bitmap = BITMAP::default();
    GetObjectW(
        hbitmap.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bitmap as *mut BITMAP as *mut _),
    );
    let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
    if width <= 0 || height <= 0 {
        return None;
    }
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let dc = CreateCompatibleDC(None);
    let lines = GetDIBits(
        dc,
        hbitmap,
        0,
        height as u32,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut info,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);
    if lines == 0 {
        return None;
    }
    // GDI hands back premultiplied BGRA; PNG wants straight RGBA.
    for px in pixels.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255) / a).min(255) as u8;
            }
        }
    }
    Some(IconRgba {
        width: width as u32,
        height: height as u32,
        rgba: pixels,
    })
}

/// Reads a COM-allocated wide string and frees it. Safe wrapper: every caller
/// passes a pointer the shell just handed back.
pub(super) fn take_pwstr(p: PWSTR) -> String {
    let s = unsafe { p.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(p.0 as _)) };
    s
}

fn apps_folder_path(app_id: &str) -> String {
    format!("shell:AppsFolder\\{app_id}")
}

fn is_file_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    (bytes.len() > 2 && bytes[1] == b':') || s.starts_with("\\\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_apps_are_launchable() {
        assert!(is_launchable(
            "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
            None
        ));
    }

    #[test]
    fn executables_and_consoles_are_launchable() {
        assert!(is_launchable("Firefox", Some("C:\\firefox\\firefox.exe")));
        assert!(is_launchable(
            "Services",
            Some("C:\\Windows\\system32\\services.msc")
        ));
    }

    #[test]
    fn custom_url_schemes_are_launchable() {
        assert!(is_launchable("A Game", Some("steam://rungameid/1")));
    }

    #[test]
    fn web_links_and_documents_are_not() {
        assert!(!is_launchable("Support", Some("http://example.com")));
        assert!(!is_launchable("Notes", Some("C:\\docs\\readme.html")));
    }

    #[test]
    fn uninstallers_and_maintenance_are_dropped() {
        assert!(!is_launchable(
            "Uninstall X",
            Some("C:\\Windows\\SysWOW64\\msiexec.exe")
        ));
        assert!(!is_launchable("Uninstall Y", Some("C:\\app\\unins000.exe")));
    }

    #[test]
    fn recognises_windows_file_paths() {
        assert!(is_file_path("C:\\a\\b.exe"));
        assert!(is_file_path("\\\\server\\share"));
        assert!(!is_file_path("steam://rungameid/1"));
        assert!(!is_file_path("Some.AUMID_abc!App"));
    }
}
