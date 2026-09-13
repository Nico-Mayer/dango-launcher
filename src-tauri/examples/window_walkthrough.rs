//! Exercises the Windows window manager against a real window.
//!
//! It opens a plain resizable window, points the manager at it through the same
//! previous-foreground the launcher records, and drives each move, reading the
//! window's visible frame back with `DWMWA_EXTENDED_FRAME_BOUNDS` to check it
//! landed exactly. No keyboard or foreground dance is needed: a window move is
//! `SetWindowPos`, not synthesized input, so the target need not be foreground.
//!
//! Usage: `cargo run --example window_walkthrough`

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("window_walkthrough only runs on Windows");
}

#[cfg(target_os = "windows")]
fn main() {
    windows_harness::run();
}

#[cfg(target_os = "windows")]
mod windows_harness {
    use std::mem::size_of;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindow,
    };

    use dango_lib::extensions::window_management::geometry::{self, Region};
    use dango_lib::platform::{self, Rect, WindowError};

    const TITLE: &str = "dango-window-walkthrough";

    struct Target {
        process: Child,
        window: HWND,
    }

    impl Target {
        fn open() -> Option<Self> {
            // A plain resizable form at a known spot, not maximised, so every
            // move has somewhere to move it from.
            let script = format!(
                "Add-Type -Name D -Namespace W -MemberDefinition '[DllImport(\"user32.dll\")] public static extern bool SetProcessDpiAwarenessContext(System.IntPtr c);'; \
                 [void][W.D]::SetProcessDpiAwarenessContext([System.IntPtr](-4)); \
                 Add-Type -AssemblyName System.Windows.Forms; \
                 $f = New-Object System.Windows.Forms.Form; \
                 $f.Text = '{TITLE}'; \
                 $f.StartPosition = 'Manual'; \
                 $f.Location = New-Object System.Drawing.Point(300, 300); \
                 $f.Size = New-Object System.Drawing.Size(700, 500); \
                 [System.Windows.Forms.Application]::Run($f)"
            );
            let process = Command::new("powershell")
                .args(["-NoProfile", "-STA", "-WindowStyle", "Normal", "-Command", &script])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()?;
            let window = wait_for_window(TITLE, Duration::from_secs(20))?;
            Some(Self { process, window })
        }

        fn close(mut self) {
            let _ = self.process.kill();
            let _ = self.process.wait();
            let deadline = Instant::now() + Duration::from_secs(5);
            while unsafe { IsWindow(self.window) } != 0 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    struct Harness {
        target: Target,
        passed: usize,
        failed: Vec<String>,
        skipped: Vec<String>,
    }

    pub fn run() {
        // Match the launcher, which Tauri runs per-monitor DPI aware. Without
        // this the process is DPI-virtualised and every coordinate it reads or
        // writes is scaled, so placement can never land where the geometry says.
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let Some(target) = Target::open() else {
            eprintln!("could not open the target window");
            return;
        };
        // Point the manager at the window the harness opened, the same slot the
        // launcher fills on show.
        platform::remember_previous_foreground(target.window as isize);

        let mut harness = Harness {
            target,
            passed: 0,
            failed: vec![],
            skipped: vec![],
        };

        harness.reads_the_target_and_its_work_area();
        harness.tiles_each_region_flush();
        harness.centre_keeps_the_size();
        harness.maximise_fills_the_work_area();
        harness.moves_to_the_next_display();
        harness.is_prompt();
        harness.no_target_is_reported();
        harness.an_elevated_target_is_refused();

        harness.report();
        harness.target.close();
    }

    impl Harness {
        fn check(&mut self, name: &str, ok: bool, detail: String) {
            if ok {
                println!("  PASS  {name}");
                self.passed += 1;
            } else {
                println!("  FAIL  {name}\n          {detail}");
                self.failed.push(format!("{name}: {detail}"));
            }
        }

        fn skip(&mut self, name: &str, why: String) {
            println!("  SKIP  {name}\n          {why}");
            self.skipped.push(format!("{name}: {why}"));
        }

        fn manager(&self) -> std::sync::Arc<dyn platform::WindowManager> {
            platform::window_manager()
        }

        fn work_area(&self) -> Rect {
            self.manager().target().expect("a target").work_area
        }

        /// Applies a frame and reads back the window's visible frame.
        fn apply(&self, frame: Rect) -> Rect {
            self.manager().place(frame).expect("place");
            std::thread::sleep(Duration::from_millis(120));
            visible_frame(self.target.window).expect("frame")
        }

        fn reads_the_target_and_its_work_area(&mut self) {
            println!("-- the target and its work area are read");
            match self.manager().target() {
                Ok(placement) => {
                    let sane = placement.work_area.width > 0
                        && placement.work_area.height > 0
                        && placement.frame.width > 0;
                    self.check(
                        "target reports a frame and a work area",
                        sane,
                        format!("{placement:?}"),
                    );
                }
                Err(error) => self.check("target reports a frame and a work area", false, format!("{error}")),
            }
        }

        fn tiles_each_region_flush(&mut self) {
            println!("-- each region lands flush against the work area");
            let area = self.work_area();
            let regions = [
                ("left half", Region::LeftHalf),
                ("right half", Region::RightHalf),
                ("top half", Region::TopHalf),
                ("bottom half", Region::BottomHalf),
                ("top-left quarter", Region::TopLeftQuarter),
                ("top-right quarter", Region::TopRightQuarter),
                ("bottom-left quarter", Region::BottomLeftQuarter),
                ("bottom-right quarter", Region::BottomRightQuarter),
                ("left third", Region::LeftThird),
                ("centre third", Region::CenterThird),
                ("right third", Region::RightThird),
            ];
            for (name, region) in regions {
                let expected = region.rect(area);
                let landed = self.apply(expected);
                self.check(
                    name,
                    close(landed, expected),
                    format!("expected {expected:?}, landed {landed:?}"),
                );
            }
        }

        fn centre_keeps_the_size(&mut self) {
            println!("-- centre keeps the window's size");
            let area = self.work_area();
            // Start from a known small size.
            let _ = self.apply(Rect { x: area.x + 50, y: area.y + 50, width: 640, height: 480 });
            let before = visible_frame(self.target.window).expect("frame");
            let centred = geometry::center(area, (before.width, before.height));
            let landed = self.apply(centred);
            self.check(
                "centre keeps size and centres",
                close(landed, centred)
                    && landed.width == before.width
                    && landed.height == before.height,
                format!("before {before:?}, expected {centred:?}, landed {landed:?}"),
            );
        }

        fn maximise_fills_the_work_area(&mut self) {
            println!("-- maximise fills the work area");
            let area = self.work_area();
            let landed = self.apply(Region::Maximize.rect(area));
            self.check(
                "maximise equals the work area",
                close(landed, area),
                format!("work area {area:?}, landed {landed:?}"),
            );
        }

        fn moves_to_the_next_display(&mut self) {
            println!("-- move to the next display");
            let displays = self.manager().displays();
            if displays.len() < 2 {
                self.skip(
                    "move to next display",
                    format!("only {} display present", displays.len()),
                );
                return;
            }
            let area = self.work_area();
            let _ = self.apply(Region::LeftHalf.rect(area));
            let frame = visible_frame(self.target.window).expect("frame");
            let Some(moved) = geometry::next_display(frame, area, &displays) else {
                self.check("move to next display", false, "geometry returned none".into());
                return;
            };
            let landed = self.apply(moved);
            // The window's centre, so the border inset does not put it just
            // outside the destination.
            let cx = landed.x + landed.width / 2;
            let cy = landed.y + landed.height / 2;
            let on_other = displays
                .iter()
                .any(|d| d.x != area.x && cx >= d.x && cx < d.x + d.width && cy >= d.y && cy < d.y + d.height);
            self.check(
                "the window moved to another display",
                on_other,
                format!("landed {landed:?}, displays {displays:?}"),
            );
        }

        fn is_prompt(&mut self) {
            println!("-- a move is prompt");
            let area = self.work_area();
            let start = Instant::now();
            self.manager().place(Region::LeftHalf.rect(area)).expect("place");
            let elapsed = start.elapsed();
            self.check(
                "place returns within 100ms",
                elapsed < Duration::from_millis(100),
                format!("place took {elapsed:?}"),
            );
            println!("          (place returned in {elapsed:?})");
        }

        fn no_target_is_reported(&mut self) {
            println!("-- no target is reported");
            platform::remember_previous_foreground(0);
            let result = self.manager().target();
            self.check(
                "target with no window is NoTarget",
                matches!(result, Err(WindowError::NoTarget)),
                format!("got {result:?}"),
            );
            platform::remember_previous_foreground(self.target.window as isize);
        }

        fn an_elevated_target_is_refused(&mut self) {
            println!("-- an elevated target is refused");
            if platform::own_integrity_level().is_some_and(|level| level > 0x2000) {
                self.skip("elevated target refused", "this harness is elevated itself".into());
                return;
            }
            let Some(elevated) = find_window("dango-elevated") else {
                self.skip(
                    "elevated target refused",
                    "no window titled *dango-elevated*; open one elevated first".into(),
                );
                return;
            };
            platform::remember_previous_foreground(elevated as isize);
            let result = self.manager().target();
            self.check(
                "an elevated target is unreachable",
                matches!(&result, Err(WindowError::Unreachable(_))),
                format!("got {result:?}"),
            );
            platform::remember_previous_foreground(self.target.window as isize);
        }

        fn report(&self) {
            println!(
                "\n{} passed, {} failed, {} skipped",
                self.passed,
                self.failed.len(),
                self.skipped.len()
            );
            for failure in &self.failed {
                println!("  {failure}");
            }
        }
    }

    /// Physical-pixel placement is meant to be exact; a pixel or two of slack
    /// absorbs a window enforcing an odd minimum without hiding a real gap.
    fn close(a: Rect, b: Rect) -> bool {
        (a.x - b.x).abs() <= 2
            && (a.y - b.y).abs() <= 2
            && (a.width - b.width).abs() <= 2
            && (a.height - b.height).abs() <= 2
    }

    fn visible_frame(hwnd: HWND) -> Option<Rect> {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let hr = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                (&mut rect as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            )
        };
        (hr == 0).then_some(Rect {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        })
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> i32 {
        let windows = &mut *(lparam as *mut Vec<HWND>);
        windows.push(hwnd);
        TRUE
    }

    fn window_title(hwnd: HWND) -> String {
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return String::new();
            }
            let mut buffer = vec![0u16; len as usize + 1];
            let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
            String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
        }
    }

    fn find_window(substring: &str) -> Option<HWND> {
        let mut windows: Vec<HWND> = Vec::new();
        unsafe {
            EnumWindows(Some(collect), &mut windows as *mut Vec<HWND> as LPARAM);
        }
        windows
            .into_iter()
            .find(|&hwnd| window_title(hwnd).contains(substring))
    }

    fn wait_for_window(title: &str, timeout: Duration) -> Option<HWND> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(hwnd) = find_window(title) {
                return Some(hwnd);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
