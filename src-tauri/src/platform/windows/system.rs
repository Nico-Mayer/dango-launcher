//! The Windows side of the `system` commands.
//!
//! A running application is a process with at least one window the task
//! switcher would show. That is the closest Windows comes to the list macOS
//! hands out directly, and it keeps agents, services, and helper processes
//! out of a list of things to quit.

use std::collections::HashSet;
use std::time::Duration;

use windows::core::{Interface, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_SUCCESS, E_UNEXPECTED, FALSE, HWND, LPARAM, STILL_ACTIVE, TRUE, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Storage::EnhancedStorage::PKEY_FileDescription;
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Shutdown::LockWorkStation;
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{
    IShellItem, IShellItem2, SHCreateItemFromParsingName, SHEmptyRecycleBinW, SHQueryRecycleBinW,
    SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHQUERYRBINFO, SIGDN_NORMALDISPLAY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetShellWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, GWL_EXSTYLE,
    GW_OWNER, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use super::apps::{init_com, process_image_path, take_pwstr, window_aumid};
use super::{RunningApp, SystemControl, SystemError};

/// How long a refused suspend takes to come back. A refusal is answered within
/// milliseconds; anything still running after this is a machine going to sleep.
const SUSPEND_GRACE: Duration = Duration::from_millis(500);

pub struct WindowsSystemControl;

impl SystemControl for WindowsSystemControl {
    fn lock(&self) -> Result<(), SystemError> {
        unsafe { LockWorkStation() }
            .map_err(|error| SystemError::Failed(format!("the system refused to lock ({error})")))
    }

    /// `SetSuspendState` does not return until the machine wakes again, so it
    /// runs on its own thread and only a prompt refusal is waited for.
    fn sleep(&self) -> Result<(), SystemError> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let suspended = unsafe { SetSuspendState(false, false, false) };
            let error = windows::core::Error::from_win32();
            let _ = tx.send((suspended, error));
        });
        match rx.recv_timeout(SUSPEND_GRACE) {
            Ok((false, error)) => Err(SystemError::Failed(format!(
                "the system refused to sleep ({error})"
            ))),
            _ => Ok(()),
        }
    }

    fn trash_count(&self) -> Result<usize, SystemError> {
        let mut info = SHQUERYRBINFO {
            cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
            ..Default::default()
        };
        unsafe { SHQueryRecycleBinW(PCWSTR::null(), &mut info) }.map_err(|error| {
            SystemError::Failed(format!("the recycle bin could not be read ({error})"))
        })?;
        Ok(info.i64NumItems.max(0) as usize)
    }

    /// The shell has its own confirmation, progress window, and sound; all
    /// three are suppressed because the launcher has already asked.
    fn empty_trash(&self) -> Result<(), SystemError> {
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        match unsafe { SHEmptyRecycleBinW(None, PCWSTR::null(), flags) } {
            Ok(()) => Ok(()),
            // Emptying a bin that is already empty is reported as E_UNEXPECTED.
            Err(error) if error.code() == E_UNEXPECTED => Ok(()),
            Err(error) => Err(SystemError::Failed(format!(
                "the recycle bin could not be emptied ({error})"
            ))),
        }
    }

    fn running_apps(&self) -> Vec<RunningApp> {
        unsafe { init_com() };
        let ours = std::process::id();
        let mut seen = HashSet::new();
        let mut apps = Vec::new();
        for hwnd in unsafe { switcher_windows() } {
            let Some(pid) = (unsafe { owning_pid(hwnd) }) else {
                continue;
            };
            if pid == ours || !seen.insert(pid) {
                continue;
            }
            let Some(identity) = (unsafe { identify(hwnd, pid) }) else {
                continue;
            };
            apps.push(RunningApp {
                id: pid.to_string(),
                name: identity.name,
                icon: Some(identity.locator),
            });
        }
        apps
    }

    /// `WM_CLOSE` to each of its switcher windows, which is what the close
    /// button sends. The application decides what to do with it, so one
    /// holding unsaved work prompts instead of dying.
    fn quit(&self, app_id: &str) -> Result<(), SystemError> {
        let pid: u32 = app_id
            .parse()
            .map_err(|_| SystemError::Failed("that is not an application".into()))?;
        let windows: Vec<HWND> = unsafe { switcher_windows() }
            .into_iter()
            .filter(|hwnd| unsafe { owning_pid(*hwnd) } == Some(pid))
            .collect();
        if windows.is_empty() {
            return Err(if unsafe { process_is_alive(pid) } {
                SystemError::Failed("that application has no window left to close".into())
            } else {
                SystemError::Gone
            });
        }
        for hwnd in windows {
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        Ok(())
    }
}

struct Identity {
    name: String,
    /// What `icon_for` renders: the application's `shell:AppsFolder` entry when
    /// its window carries one, otherwise the executable itself.
    locator: String,
}

/// Named the way the taskbar names it where possible. An AppUserModelID the
/// apps folder knows means a packaged or registered app, and the folder has
/// its proper name and icon. A store app carries the id on its window; a
/// packaged desktop app such as Terminal carries it on its process. Anything
/// else is named by its executable's file description, falling back to the
/// file name.
unsafe fn identify(hwnd: HWND, pid: u32) -> Option<Identity> {
    if let Some(aumid) = window_aumid(hwnd).or_else(|| process_aumid(pid)) {
        let locator = format!("shell:AppsFolder\\{aumid}");
        if let Some(item) = shell_item(&locator) {
            if let Some(name) = display_name(&item) {
                return Some(Identity { name, locator });
            }
        }
    }
    let path = process_image_path(pid)?;
    let name = shell_item(&path)
        .and_then(|item| {
            item.cast::<IShellItem2>()
                .ok()?
                .GetString(&PKEY_FileDescription)
                .ok()
                .map(take_pwstr)
        })
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| file_stem(&path));
    Some(Identity {
        name,
        locator: path,
    })
}

unsafe fn process_aumid(pid: u32) -> Option<String> {
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buffer = [0u16; 256];
    let mut len = buffer.len() as u32;
    let status = GetApplicationUserModelId(process, &mut len, Some(PWSTR(buffer.as_mut_ptr())));
    let _ = CloseHandle(process);
    if status != ERROR_SUCCESS || len < 2 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..len as usize - 1]))
}

unsafe fn shell_item(parsing_name: &str) -> Option<IShellItem> {
    SHCreateItemFromParsingName(PCWSTR(HSTRING::from(parsing_name).as_ptr()), None).ok()
}

unsafe fn display_name(item: &IShellItem) -> Option<String> {
    item.GetDisplayName(SIGDN_NORMALDISPLAY)
        .ok()
        .map(take_pwstr)
        .filter(|name| !name.is_empty())
}

fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

struct Collector {
    hwnds: Vec<HWND>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let collector = &mut *(lparam.0 as *mut Collector);
    collector.hwnds.push(hwnd);
    TRUE
}

/// The windows Alt-Tab would offer: visible, unowned, titled, not a tool
/// window, not cloaked, and not the desktop. Cloaking is how Windows parks
/// suspended store apps and windows on other virtual desktops; the desktop
/// window is excluded by identity because closing it opens the shutdown dialog.
unsafe fn switcher_windows() -> Vec<HWND> {
    let mut collector = Collector { hwnds: Vec::new() };
    let _ = EnumWindows(Some(collect), LPARAM(&mut collector as *mut _ as isize));
    let shell = GetShellWindow();
    collector
        .hwnds
        .into_iter()
        .filter(|&hwnd| {
            if hwnd == shell || !IsWindowVisible(hwnd).as_bool() {
                return false;
            }
            if GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.0.is_null()) {
                return false;
            }
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            if style & WS_EX_TOOLWINDOW.0 != 0 && style & WS_EX_APPWINDOW.0 == 0 {
                return false;
            }
            !is_cloaked(hwnd) && GetWindowTextLengthW(hwnd) > 0
        })
        .collect()
}

unsafe fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut _,
        std::mem::size_of::<u32>() as u32,
    )
    .is_ok()
        && cloaked != 0
}

/// The process the user would name. Store apps run inside a frame owned by
/// `ApplicationFrameHost`, so the frame's own process would lump every store
/// app together; the hosted application is the child window from another
/// process.
unsafe fn owning_pid(hwnd: HWND) -> Option<u32> {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return None;
    }
    let is_frame_host = process_image_path(pid)
        .is_some_and(|path| file_stem(&path).eq_ignore_ascii_case("ApplicationFrameHost"));
    if !is_frame_host {
        return Some(pid);
    }
    Some(hosted_pid(hwnd, pid).unwrap_or(pid))
}

struct Hosted {
    frame_pid: u32,
    found: Option<u32>,
}

unsafe extern "system" fn find_hosted(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let hosted = &mut *(lparam.0 as *mut Hosted);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid != 0 && pid != hosted.frame_pid {
        hosted.found = Some(pid);
        return FALSE;
    }
    TRUE
}

unsafe fn hosted_pid(frame: HWND, frame_pid: u32) -> Option<u32> {
    let mut hosted = Hosted {
        frame_pid,
        found: None,
    };
    let _ = EnumChildWindows(
        Some(frame),
        Some(find_hosted),
        LPARAM(&mut hosted as *mut _ as isize),
    );
    hosted.found
}

unsafe fn process_is_alive(pid: u32) -> bool {
    let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return false;
    };
    let mut code = 0u32;
    let alive = GetExitCodeProcess(process, &mut code).is_ok() && code == STILL_ACTIVE.0 as u32;
    let _ = CloseHandle(process);
    alive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_executable_path_falls_back_to_its_filename() {
        assert_eq!(file_stem("C:\\Program Files\\App\\app.exe"), "app");
    }

    /// Needs a desktop session: there is no window station to enumerate in CI
    /// and nothing running to quit.
    mod live {
        use super::*;

        #[test]
        #[ignore]
        fn running_applications_are_listed_without_ourselves() {
            let apps = WindowsSystemControl.running_apps();
            assert!(!apps.is_empty(), "something must be running");
            let ours = std::process::id().to_string();
            assert!(apps.iter().all(|app| app.id != ours));
            assert!(apps
                .iter()
                .all(|app| !app.id.is_empty() && !app.name.is_empty()));
            for app in &apps {
                let locator = app.icon.as_deref().expect("every app has an icon locator");
                eprintln!("{:>7}  {:<40} {locator}", app.id, app.name);
                assert!(
                    crate::platform::icon_for(locator, 32).is_some(),
                    "{} has no icon at {locator}",
                    app.name
                );
            }
        }

        #[test]
        #[ignore]
        fn the_recycle_bin_can_be_counted() {
            let count = WindowsSystemControl.trash_count().unwrap();
            eprintln!("recycle bin holds {count} items");
        }

        /// Calculator is a store app hosted in ApplicationFrameHost, so this
        /// covers the harder case: the listed process must be Calculator's own,
        /// and closing the frame must end it.
        #[test]
        #[ignore]
        fn a_running_application_is_asked_to_quit() {
            fn instances() -> usize {
                let output = std::process::Command::new("tasklist")
                    .args(["/FI", "IMAGENAME eq CalculatorApp.exe", "/NH"])
                    .output()
                    .unwrap();
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| line.contains("CalculatorApp.exe"))
                    .count()
            }
            assert_eq!(instances(), 0, "Calculator must not be running already");

            std::process::Command::new("cmd")
                .args(["/C", "start", "calculator:"])
                .status()
                .unwrap();
            let mut app = None;
            for _ in 0..40 {
                std::thread::sleep(Duration::from_millis(250));
                app = WindowsSystemControl.running_apps().into_iter().find(|app| {
                    app.icon
                        .as_deref()
                        .is_some_and(|icon| icon.contains("Microsoft.WindowsCalculator"))
                });
                if app.is_some() {
                    break;
                }
            }
            let app = app.expect("Calculator must be listed as running");
            assert_eq!(instances(), 1, "Calculator did not start");
            WindowsSystemControl.quit(&app.id).unwrap();

            for _ in 0..40 {
                std::thread::sleep(Duration::from_millis(250));
                if instances() == 0 {
                    break;
                }
            }
            assert_eq!(instances(), 0, "Calculator was not asked to quit");
        }

        /// A form that cancels its own close stands in for an application
        /// prompting about unsaved work: the request must reach it, it must
        /// still be running afterwards, and neither is a failure.
        #[test]
        #[ignore]
        fn an_application_that_refuses_to_close_is_left_running() {
            const TITLE: &str = "Dango close probe";
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; \
                 $f = New-Object Windows.Forms.Form; $f.Text = '{TITLE}'; \
                 $f.Add_FormClosing({{ $_.Cancel = $true; $f.Text = '{TITLE} refused' }}); \
                 [Windows.Forms.Application]::Run($f)"
            );
            let mut probe = std::process::Command::new("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
                .spawn()
                .unwrap();
            let id = probe.id().to_string();

            let mut listed = false;
            for _ in 0..40 {
                std::thread::sleep(Duration::from_millis(250));
                listed = WindowsSystemControl
                    .running_apps()
                    .iter()
                    .any(|app| app.id == id);
                if listed {
                    break;
                }
            }
            assert!(listed, "the probe window must be listed as running");

            assert!(WindowsSystemControl.quit(&id).is_ok());
            std::thread::sleep(Duration::from_millis(1500));
            assert!(
                probe.try_wait().unwrap().is_none(),
                "a refusal must not end the application"
            );
            assert!(
                window_titles(probe.id())
                    .iter()
                    .any(|t| t.ends_with("refused")),
                "the close request never reached the window"
            );
            let _ = probe.kill();
        }

        fn window_titles(pid: u32) -> Vec<String> {
            use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
            unsafe {
                let mut collector = Collector { hwnds: Vec::new() };
                let _ = EnumWindows(Some(collect), LPARAM(&mut collector as *mut _ as isize));
                collector
                    .hwnds
                    .into_iter()
                    .filter(|&hwnd| {
                        let mut owner = 0u32;
                        GetWindowThreadProcessId(hwnd, Some(&mut owner));
                        owner == pid
                    })
                    .map(|hwnd| {
                        let mut buffer = [0u16; 256];
                        let len = GetWindowTextW(hwnd, &mut buffer) as usize;
                        String::from_utf16_lossy(&buffer[..len])
                    })
                    .collect()
            }
        }

        #[test]
        #[ignore]
        fn quitting_something_that_is_not_running_reports_it_as_gone() {
            assert!(matches!(
                WindowsSystemControl.quit("4000000"),
                Err(SystemError::Gone)
            ));
        }
    }
}
