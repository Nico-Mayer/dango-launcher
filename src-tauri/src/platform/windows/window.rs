//! The Windows half of window management.
//!
//! Two Windows facts shape it. `GetWindowRect` returns a rectangle larger than
//! what the user sees, because it includes the invisible resize border, so
//! placing by it leaves a gap; `DWMWA_EXTENDED_FRAME_BOUNDS` gives the visible
//! frame, and the difference between the two is the inset a placement must undo.
//! And everything is in physical pixels across one virtual desktop, so the work
//! area, the frame, and the target all share coordinates and nothing converts.

use std::mem::size_of;

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsZoomed, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER, SW_RESTORE,
};

use super::text::{is_out_of_reach, previous_foreground};
use crate::platform::{Placement, Rect, WindowError, WindowManager};

pub struct WindowsWindowManager;

impl WindowManager for WindowsWindowManager {
    fn target(&self) -> Result<Placement, WindowError> {
        let hwnd = previous_foreground().ok_or(WindowError::NoTarget)?;
        if is_out_of_reach(hwnd) {
            return Err(WindowError::Unreachable(
                "Dango can't move windows of programs running as administrator.".into(),
            ));
        }
        let frame = visible_frame(hwnd).ok_or_else(|| {
            eprintln!("[dango] could not read the window's frame");
            WindowError::Failed(crate::platform::WINDOW_READ_FAILED.into())
        })?;
        let work_area = work_area(hwnd).ok_or_else(|| {
            eprintln!("[dango] could not read the display");
            WindowError::Failed(crate::platform::WINDOW_READ_FAILED.into())
        })?;
        Ok(Placement { frame, work_area })
    }

    fn displays(&self) -> Vec<Rect> {
        let mut areas: Vec<Rect> = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect_work_area),
                &mut areas as *mut Vec<Rect> as LPARAM,
            );
        }
        // A stable left-to-right, top-to-bottom order so move-to-next-display is
        // predictable rather than dependent on enumeration order.
        areas.sort_by_key(|r| (r.x, r.y));
        areas
    }

    fn place(&self, frame: Rect) -> Result<(), WindowError> {
        let hwnd = previous_foreground().ok_or(WindowError::NoTarget)?;
        if is_out_of_reach(hwnd) {
            return Err(WindowError::Unreachable(
                "Dango can't move windows of programs running as administrator.".into(),
            ));
        }

        // A maximised window ignores a move, so drop it back to a normal state
        // first; then the tile takes effect.
        if unsafe { IsZoomed(hwnd) } == TRUE {
            unsafe { ShowWindow(hwnd, SW_RESTORE) };
        }

        // Undo the invisible border: SetWindowPos moves the outer window rect,
        // but `frame` is where the visible edges should land.
        let inset = border_inset(hwnd).unwrap_or(Inset::ZERO);
        let x = frame.x - inset.left;
        let y = frame.y - inset.top;
        let width = frame.width + inset.left + inset.right;
        let height = frame.height + inset.top + inset.bottom;

        let ok = unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        if ok == 0 {
            return Err(WindowError::Failed(
                "That window didn't move. It may have a fixed size.".into(),
            ));
        }
        Ok(())
    }
}

fn rect_to_frame(r: RECT) -> Rect {
    Rect {
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}

/// The visible frame, corrected for the invisible resize border.
fn visible_frame(hwnd: HWND) -> Option<Rect> {
    dwm_frame(hwnd).map(rect_to_frame)
}

fn dwm_frame(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut rect as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
    };
    if hr == 0 {
        Some(rect)
    } else {
        None
    }
}

struct Inset {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Inset {
    const ZERO: Inset = Inset {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
}

/// How much larger the outer window rect is than the visible frame on each edge.
fn border_inset(hwnd: HWND) -> Option<Inset> {
    let mut outer = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetWindowRect(hwnd, &mut outer) } == 0 {
        return None;
    }
    let visible = dwm_frame(hwnd)?;
    Some(Inset {
        left: visible.left - outer.left,
        top: visible.top - outer.top,
        right: outer.right - visible.right,
        bottom: outer.bottom - visible.bottom,
    })
}

fn work_area(hwnd: HWND) -> Option<Rect> {
    let monitor: HMONITOR = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return None;
    }
    monitor_work_area(monitor)
}

fn monitor_work_area(monitor: HMONITOR) -> Option<Rect> {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        rcWork: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        dwFlags: 0,
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    Some(rect_to_frame(info.rcWork))
}

unsafe extern "system" fn collect_work_area(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> i32 {
    let areas = &mut *(data as *mut Vec<Rect>);
    if let Some(area) = monitor_work_area(monitor) {
        areas.push(area);
    }
    TRUE
}
