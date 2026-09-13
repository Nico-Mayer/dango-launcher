//! Exercises the platform window manager against a real window.
//!
//! On Windows it opens a plain resizable window, points the manager at it
//! through the same previous-foreground the launcher records, and drives each
//! move, reading the window's visible frame back with
//! `DWMWA_EXTENDED_FRAME_BOUNDS` to check it landed exactly. No keyboard or
//! foreground dance is needed: a window move is `SetWindowPos`, not synthesized
//! input, so the target need not be foreground.
//!
//! On macOS the target is a scratch TextEdit document, or a Finder window with
//! `cargo run --example window_walkthrough -- finder`, because the manager acts
//! on the frontmost application's focused window and there is no handle to
//! point it at. The harness itself is not an application, so it never becomes
//! frontmost and never steals the target. Frames are read back by asking System
//! Events, so the answer comes from a different process than the one that wrote
//! it and a write that reported success while moving nothing is still caught.
//! It needs the Accessibility permission granted to the example binary, not to
//! Dango.
//!
//! Usage: `cargo run --example window_walkthrough`

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {
    eprintln!("window_walkthrough only runs on Windows and macOS");
}

#[cfg(target_os = "windows")]
fn main() {
    windows_harness::run();
}

#[cfg(target_os = "macos")]
fn main() {
    macos_harness::run();
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
    use dango_lib::text::HereIsFine;

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
        harness.the_new_sizes_land();
        harness.grows_and_shrinks_by_steps();
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
            platform::window_manager(std::sync::Arc::new(HereIsFine))
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

        fn the_new_sizes_land(&mut self) {
            println!("-- the new absolute sizes land centred");
            let area = self.work_area();
            for (name, expected) in [
                ("almost maximise", geometry::almost_maximize(area)),
                ("reasonable size", geometry::reasonable_size(area)),
                ("centre half", geometry::center_half(area)),
            ] {
                let landed = self.apply(expected);
                self.check(name, close(landed, expected), format!("expected {expected:?}, landed {landed:?}"));
            }
        }

        fn grows_and_shrinks_by_steps(&mut self) {
            println!("-- make larger and smaller step and clamp");
            let area = self.work_area();
            let _ = self.apply(geometry::reasonable_size(area));
            let before = visible_frame(self.target.window).expect("frame");
            let larger = self.apply(geometry::step(before, area, geometry::Step::Larger));
            self.check(
                "make larger grows the window",
                larger.width > before.width && larger.height > before.height,
                format!("before {before:?}, larger {larger:?}"),
            );
            let mut frame = larger;
            for _ in 0..40 {
                frame = self.apply(geometry::step(frame, area, geometry::Step::Larger));
            }
            self.check(
                "make larger clamps to the work area",
                close(frame, area),
                format!("work area {area:?}, landed {frame:?}"),
            );
            for _ in 0..40 {
                frame = self.apply(geometry::step(frame, area, geometry::Step::Smaller));
            }
            self.check(
                "make smaller clamps to the minimum",
                (frame.width - 400).abs() <= 2 && (frame.height - 300).abs() <= 2,
                format!("landed {frame:?}"),
            );
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

#[cfg(target_os = "macos")]
mod macos_harness {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSWorkspace};
    use objc2_foundation::MainThreadMarker;

    use dango_lib::extensions::window_management::geometry::{self, Region};
    use dango_lib::platform::{self, Rect, WindowError};
    use dango_lib::text::HereIsFine;

    const SCRATCH: &str = "/tmp/dango-window-walkthrough.txt";

    /// Long enough for the target to take a move and redraw. Accessibility
    /// writes return before the application has finished applying them.
    const SETTLE: Duration = Duration::from_millis(150);

    /// Which application's window is being pushed around. Two are offered
    /// because "it works" has to mean more than one application's idea of what
    /// a window is: TextEdit is a plain document window, Finder's is a browser
    /// with a sidebar and its own minimum size.
    struct App {
        process: &'static str,
        bundle: &'static str,
        open: [&'static str; 2],
        /// Whether closing the last window leaves the application with none.
        /// Finder always keeps one: the desktop is a window, and it stays
        /// focused, so closing everything never produces "no window to move".
        closes_to_nothing: bool,
    }

    const TEXT_EDIT: App = App {
        process: "TextEdit",
        bundle: "com.apple.TextEdit",
        open: ["TextEdit", SCRATCH],
        closes_to_nothing: true,
    };
    const FINDER: App = App {
        process: "Finder",
        bundle: "com.apple.finder",
        open: ["Finder", "/tmp"],
        closes_to_nothing: false,
    };

    struct Harness {
        app: App,
        passed: usize,
        failed: Vec<String>,
        skipped: Vec<String>,
    }

    pub fn run() {
        if !unsafe { objc2_application_services::AXIsProcessTrusted() } {
            without_the_permission();
            return;
        }
        become_an_application();
        let app = match std::env::args().nth(1).unwrap_or_default().as_str() {
            "finder" => FINDER,
            "textedit" | "" => TEXT_EDIT,
            other => {
                eprintln!("unknown target {other}; use textedit or finder");
                return;
            }
        };
        println!("target: {}\n", app.process);
        if !open_target(&app) {
            eprintln!("could not bring a {} window to the front", app.process);
            return;
        }

        let mut harness = Harness {
            app,
            passed: 0,
            failed: vec![],
            skipped: vec![],
        };

        harness.reads_the_target_and_its_work_area();
        harness.on_every_display();
        harness.moves_to_the_next_display();
        harness.is_prompt();
        harness.no_focused_window_is_reported();

        harness.report();
        harness.close_target();
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

        /// The harness runs on the main thread, so the `NSScreen` hop is inline.
        fn manager(&self) -> std::sync::Arc<dyn platform::WindowManager> {
            platform::window_manager(std::sync::Arc::new(HereIsFine))
        }

        fn work_area(&self) -> Rect {
            self.manager().target().expect("a target").work_area
        }

        /// Every placement check, once per display. A second display is where
        /// the coordinate flip and the reserved areas can differ, so running the
        /// suite only where the window happened to start would miss it.
        fn on_every_display(&mut self) {
            let displays = self.manager().displays();
            for (position, area) in displays.iter().enumerate() {
                println!("\n== display {} of {}: {area:?}", position + 1, displays.len());
                self.apply(geometry::center(*area, (800, 600)));
                let landed_on = self.work_area();
                if landed_on != *area {
                    self.check(
                        "the window can be put on this display",
                        false,
                        format!("asked for {area:?}, ended up on {landed_on:?}"),
                    );
                    continue;
                }
                self.the_work_area_leaves_the_menu_bar(*area);
                self.tiles_each_region_flush(*area);
                self.centre_keeps_the_size(*area);
                self.maximise_fills_the_work_area(*area);
            }
        }

        /// Applies a frame and reads the window back through AppleScript.
        fn apply(&self, frame: Rect) -> Rect {
            self.manager().place(frame).expect("place");
            std::thread::sleep(SETTLE);
            self.bounds().expect("bounds")
        }

        /// The window's frame as System Events reports it. Direct Apple Events
        /// to the application were tried first and time out without an
        /// Automation grant; System Events needs only the Accessibility one this
        /// already requires.
        ///
        /// Asking for the standard window rather than window 1: a run of quick
        /// moves makes the system's own tiling hint flash up, and while it is
        /// there it is window 1, so the read comes back as an eighty-four by
        /// seventy-seven rectangle that is not the target at all.
        fn bounds(&self) -> Option<Rect> {
            let window = "(first window whose subrole is \"AXStandardWindow\")";
            rect_from(&format!(
                "tell application \"System Events\" to tell process \"{}\" to get \
                 {{value of attribute \"AXPosition\" of {window}, \
                   value of attribute \"AXSize\" of {window}}}",
                self.app.process
            ))
        }

        /// The menu bar's own frame, in the same coordinates a window frame uses.
        fn menu_bar(&self) -> Option<Rect> {
            rect_from(&format!(
                "tell application \"System Events\" to tell process \"{}\" to get \
                 {{value of attribute \"AXPosition\" of menu bar 1, \
                   value of attribute \"AXSize\" of menu bar 1}}",
                self.app.process
            ))
        }

        /// Closes the scratch window and leaves the application running, so a
        /// document the user had open is not taken down with it. TextEdit's
        /// scratch file is saved and unchanged, so nothing asks about it.
        fn close_target(&self) {
            let _ = osascript(
                "tell application \"System Events\" to keystroke \"w\" using command down",
            );
        }

        fn reads_the_target_and_its_work_area(&mut self) {
            println!("-- the target and its work area are read");
            match self.manager().target() {
                Ok(placement) => {
                    let reported = self.bounds();
                    self.check(
                        "target reports the frontmost window's own frame",
                        reported.is_some_and(|bounds| close(bounds, placement.frame)),
                        format!("accessibility {:?}, applescript {reported:?}", placement.frame),
                    );
                }
                Err(error) => self.check(
                    "target reports the frontmost window's own frame",
                    false,
                    format!("{error}"),
                ),
            }
        }

        /// The one assertion that is not circular. Every other check compares a
        /// placement against the work area this same code computed, so a work
        /// area flipped upside down would agree with itself.
        ///
        /// The menu bar is the independent witness. The system reports it as an
        /// element of its own, and where it sits is a fact about the screen
        /// rather than something this code derived: it occupies the top of the
        /// primary screen, so that screen's work area has to start exactly where
        /// the menu bar ends. An upside-down work area would start at zero.
        fn the_work_area_leaves_the_menu_bar(&mut self, area: Rect) {
            println!("-- the work area leaves the menu bar");
            let Some(menu_bar) = self.menu_bar() else {
                self.check("the menu bar is reported", false, "none".into());
                return;
            };
            // The menu bar follows the frontmost application's window, so this
            // is the menu bar of the display the window is on.
            self.check(
                "the work area starts where the menu bar ends",
                menu_bar.x == area.x && (area.y - (menu_bar.y + menu_bar.height)).abs() <= 2,
                format!("menu bar {menu_bar:?}, work area {area:?}"),
            );
        }

        fn tiles_each_region_flush(&mut self, area: Rect) {
            println!("-- each region lands flush against the work area");
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

        fn centre_keeps_the_size(&mut self, area: Rect) {
            println!("-- centre keeps the window's size");
            let before = self.apply(Rect {
                x: area.x + 50,
                y: area.y + 50,
                width: 640,
                height: 480,
            });
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

        fn maximise_fills_the_work_area(&mut self, area: Rect) {
            println!("-- maximise fills the work area");
            let landed = self.apply(Region::Maximize.rect(area));
            self.check(
                "maximise equals the work area",
                close(landed, area),
                format!("work area {area:?}, landed {landed:?}"),
            );
        }

        fn moves_to_the_next_display(&mut self) {
            println!("\n-- move to the next display");
            let displays = self.manager().displays();
            if displays.len() < 2 {
                self.skip(
                    "move to next display",
                    format!("only {} display present", displays.len()),
                );
                return;
            }
            let area = self.work_area();
            let frame = self.apply(Region::LeftHalf.rect(area));
            let Some(moved) = geometry::next_display(frame, area, &displays) else {
                self.check("move to next display", false, "geometry returned none".into());
                return;
            };
            let landed = self.apply(moved);
            let index = displays.iter().position(|d| *d == area).expect("this display");
            let destination = displays[(index + 1) % displays.len()];
            // The same left half, on the next display's work area. The tolerance
            // is wider than a placement's: the relative region is recomputed
            // from fractions, so a pixel of slack in the frame it started from
            // is magnified by the ratio between the two displays.
            let expected = Region::LeftHalf.rect(destination);
            self.check(
                "a left-half window is a left-half window on the next display",
                within(landed, moved, 2) && within(moved, expected, 8),
                format!("expected {expected:?}, asked {moved:?}, landed {landed:?}, displays {displays:?}"),
            );
        }

        fn is_prompt(&mut self) {
            println!("-- a move is prompt");
            let area = self.work_area();
            let start = Instant::now();
            self.manager()
                .place(Region::LeftHalf.rect(area))
                .expect("place");
            let elapsed = start.elapsed();
            self.check(
                "place returns within 100ms",
                elapsed < Duration::from_millis(100),
                format!("place took {elapsed:?}"),
            );
            println!("          (place returned in {elapsed:?})");
        }

        /// Closing the document leaves TextEdit frontmost with nothing focused,
        /// which is the macOS shape of "there is no window to move".
        fn no_focused_window_is_reported(&mut self) {
            println!("-- no focused window is reported");
            if !self.app.closes_to_nothing {
                self.close_target();
                self.skip(
                    "no focused window is NoTarget",
                    format!("{}'s desktop stays focused with no window open", self.app.process),
                );
                return;
            }
            self.close_target();
            std::thread::sleep(SETTLE);
            if frontmost_bundle().as_deref() != Some(self.app.bundle) {
                self.skip(
                    "no focused window is NoTarget",
                    format!("{} stopped being frontmost after the close", self.app.process),
                );
                return;
            }
            let result = self.manager().target();
            self.check(
                "no focused window is NoTarget",
                matches!(result, Err(WindowError::NoTarget)),
                format!("got {result:?}"),
            );
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

    /// Logical points, so placement should be exact. A pixel or two of slack
    /// absorbs an application enforcing a minimum or snapping to a text grid
    /// without hiding a real gap.
    fn close(a: Rect, b: Rect) -> bool {
        within(a, b, 2)
    }

    fn within(a: Rect, b: Rect, slack: i32) -> bool {
        (a.x - b.x).abs() <= slack
            && (a.y - b.y).abs() <= slack
            && (a.width - b.width).abs() <= slack
            && (a.height - b.height).abs() <= slack
    }


    /// What an untrusted process gets. Running a copy of this binary from a
    /// path the permission was never granted to exercises the same branch a
    /// revoked permission does, without taking the grant away from anything
    /// else. Every command has to say what is wrong and offer the prompt, not
    /// quietly do nothing.
    fn without_the_permission() {
        println!("-- without the Accessibility permission");
        let manager = platform::window_manager(std::sync::Arc::new(HereIsFine));
        let target = manager.target();
        let placed = manager.place(Rect { x: 0, y: 0, width: 800, height: 600 });
        println!("  target: {target:?}");
        println!("  place:  {placed:?}");
        fn explained<T>(result: &Result<T, WindowError>) -> bool {
            matches!(result, Err(WindowError::Failed(message)) if message.contains("Accessibility"))
        }
        if explained(&target) && explained(&placed) {
            println!("  PASS  both explain the missing permission and prompt for it");
        } else {
            println!("  FAIL  the missing permission was not explained");
        }
        println!("\nGrant Accessibility to:");
        println!("  {}", std::env::current_exe().unwrap().display());
        println!("in System Settings > Privacy & Security > Accessibility, then rerun.");
    }

    /// `NSScreen` answers differently to a process that is not an AppKit
    /// application: it reports the menu bar inset of the display currently
    /// showing one and nothing for the others, so a second display's work area
    /// comes back thirty points too tall and every placement there is pushed
    /// down by the window server. Waking the shared application up fixes it, and
    /// it is the state Dango is always in. Accessory, so the harness still never
    /// activates and never becomes the target itself.
    fn become_an_application() {
        let mtm = MainThreadMarker::new().expect("the main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        app.finishLaunching();
    }

    fn open_target(app: &App) -> bool {
        let _ = std::fs::write(SCRATCH, "dango window walkthrough\n");
        // `open` activates as well, so no separate activation is needed.
        let _ = Command::new("open").arg("-a").args(app.open).status();

        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if frontmost_bundle().as_deref() == Some(app.bundle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    fn frontmost_bundle() -> Option<String> {
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()?
            .bundleIdentifier()
            .map(|id| id.to_string())
    }

    fn rect_from(script: &str) -> Option<Rect> {
        let output = osascript(script)?;
        let numbers: Vec<i32> = output
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect();
        match numbers[..] {
            [x, y, width, height] => Some(Rect { x, y, width, height }),
            _ => None,
        }
    }

    fn osascript(script: &str) -> Option<String> {
        let output = Command::new("osascript").args(["-e", script]).output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}
