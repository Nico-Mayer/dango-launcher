//! The Windows walkthrough.
//!
//! The target is a text box the harness opens itself: a WinForms window whose
//! only control is a multi-line `TextBox`, which is a real Win32 edit control.
//! That choice does two jobs at once. It is a native application and a .NET one
//! in the same window, and it can be read back with `WM_GETTEXT` from another
//! process, so checking what landed never touches the clipboard the checks are
//! about.
//!
//! Two rules learned on macOS hold here too. Dango must not be running, or two
//! clipboard owners fight. And nothing is read from a window until the harness
//! has confirmed that window is the foreground one; assuming it was is how the
//! macOS harness copied from the wrong window twice.
//!
//! One rule is new. The elevated-target check only means something when this
//! process is not elevated itself, so the harness reports its own integrity
//! level and skips that check when it outranks what it is testing against. A
//! window titled `dango-elevated` is used as the elevated target when present.
//!
//! Usage: `cargo run --example walkthrough`

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use windows_sys::Win32::Foundation::{HWND, LPARAM, TRUE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindow, SendMessageW, GW_CHILD, WM_GETTEXT, WM_GETTEXTLENGTH,
    WM_SETTEXT,
};

use dango_lib::extensions::clipboard::{Content, CrateClipboard};
use dango_lib::platform::{self, WindowsHandoff};
use dango_lib::text::{Handoff, HereIsFine, Launcher, OwnWrites, TextError, TextExchange};

const TARGET_TITLE: &str = "dango-walkthrough-target";
const DECOY_TITLE: &str = "dango-walkthrough-decoy";
const ELEVATED_TITLE: &str = "dango-elevated";
const USER_CLIPBOARD: &str = "dango-walkthrough-user-clipboard";
const EM_SETSEL: u32 = 0x00B1;
const MEDIUM_INTEGRITY: u32 = 0x2000;

struct NoLauncher;

impl Launcher for NoLauncher {
    fn dismiss(&self) {}
}

#[derive(Default)]
struct Declared(Mutex<Vec<Content>>);

impl OwnWrites for Declared {
    fn expect(&self, content: &Content) {
        self.0.lock().unwrap().push(content.clone());
    }
}

impl Declared {
    fn count(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

/// A window the harness opened and the edit control inside it.
struct TextBox {
    process: Child,
    window: HWND,
    edit: HWND,
}

impl TextBox {
    fn open(title: &str, x: i32) -> Option<Self> {
        // The form asserts the foreground itself on show. A process that is
        // starting up is allowed one SetForegroundWindow, which is how the
        // target stands in for "the app the user was in had focus before the
        // launcher appeared" even over a fullscreen window that is holding it.
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             Add-Type -Name Fg -Namespace W -MemberDefinition '[DllImport(\"user32.dll\")] public static extern bool SetForegroundWindow(System.IntPtr h);'; \
             $f = New-Object System.Windows.Forms.Form; \
             $f.Text = '{title}'; $f.Width = 560; $f.Height = 360; \
             $f.StartPosition = 'Manual'; $f.Location = New-Object System.Drawing.Point({x}, 120); \
             $t = New-Object System.Windows.Forms.TextBox; \
             $t.Multiline = $true; $t.AcceptsReturn = $true; $t.Dock = 'Fill'; \
             $t.Font = New-Object System.Drawing.Font('Consolas', 12); \
             $f.Controls.Add($t); \
             $f.Add_Shown({{ $f.Activate(); [void][W.Fg]::SetForegroundWindow($f.Handle); $t.Focus() }}); \
             [System.Windows.Forms.Application]::Run($f)"
        );
        let process = Command::new("powershell")
            .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let window = wait_for_window(|t| t == title, Duration::from_secs(20))?;
        let edit = unsafe { GetWindow(window, GW_CHILD) };
        if edit.is_null() {
            return None;
        }
        Some(Self {
            process,
            window,
            edit,
        })
    }

    fn text(&self) -> String {
        text_of(self.edit)
    }

    fn set_text(&self, text: &str) {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            SendMessageW(self.edit, WM_SETTEXT, 0, wide.as_ptr() as LPARAM);
        }
    }

    fn select_all(&self) {
        unsafe {
            SendMessageW(self.edit, EM_SETSEL, 0, -1);
        }
    }

    fn close(mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        let gone = Instant::now() + Duration::from_secs(5);
        while unsafe { IsWindow(self.window) } != 0 && Instant::now() < gone {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

struct Harness {
    enigo: Enigo,
    clipboard: ClipboardContext,
    exchange: Arc<TextExchange>,
    handoff: WindowsHandoff,
    declared: Arc<Declared>,
    target: TextBox,
    passed: usize,
    failed: Vec<String>,
    skipped: Vec<String>,
}

pub fn run() {
    if dango_is_running() {
        eprintln!("Dango is running. Quit it first: two clipboard owners fight.");
        return;
    }
    let integrity = platform::own_integrity_level();
    match integrity {
        Some(level) if level > MEDIUM_INTEGRITY => {
            println!("running elevated (integrity {level:#x}); the elevated-target check will be skipped.");
            println!("for that check, run this from a medium-integrity shell.");
        }
        Some(level) => println!("running at integrity {level:#x}"),
        None => println!("could not read own integrity level"),
    }

    let Some(source) = CrateClipboard::new() else {
        eprintln!("no clipboard");
        return;
    };
    let declared = Arc::new(Declared::default());
    let Some(exchange) = platform::text_exchange(
        source,
        declared.clone(),
        Arc::new(NoLauncher),
        Arc::new(HereIsFine),
    ) else {
        eprintln!("no key injection");
        return;
    };
    let Ok(clipboard) = ClipboardContext::new() else {
        eprintln!("no clipboard");
        return;
    };
    let Ok(enigo) = Enigo::new(&Settings::default()) else {
        eprintln!("no key injection");
        return;
    };

    println!("This takes over the keyboard and the foreground for about a minute.");
    println!("Ctrl-C now to abort.");
    for remaining in (1..=3).rev() {
        println!("  starting in {remaining}...");
        std::thread::sleep(Duration::from_secs(1));
    }
    println!();

    let Some(target) = TextBox::open(TARGET_TITLE, 80) else {
        eprintln!("could not open the target text box");
        return;
    };

    let mut harness = Harness {
        enigo,
        clipboard,
        exchange: Arc::new(exchange),
        handoff: WindowsHandoff,
        declared,
        target,
        passed: 0,
        failed: vec![],
        skipped: vec![],
    };

    if !harness.target_is_ready() {
        harness.target.close();
        return;
    }
    harness.warm_up();

    harness.text_reaches_the_application();
    harness.the_users_clipboard_survives();
    harness.an_image_on_the_clipboard_survives();
    harness.both_writes_are_declared();
    harness.the_caret_lands_where_asked();
    harness.the_selection_is_read();
    harness.an_empty_selection_is_reported_as_empty();
    harness.text_appears_promptly();
    harness.the_target_is_brought_forward_first();
    harness.a_vanished_target_is_reported();
    harness.no_target_is_reported();
    harness.the_paste_acknowledgement_is_measured();
    harness.an_elevated_target_is_refused(integrity);

    let _ = harness.clipboard.set_text(String::new());
    harness.report();
    harness.target.close();
}

impl Harness {
    fn brief(text: &str) -> String {
        let cleaned: String = text.chars().take(80).collect();
        if text.chars().count() > 80 {
            format!("{cleaned:?}... ({} chars total)", text.chars().count())
        } else {
            format!("{cleaned:?}")
        }
    }

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

    fn target_is_ready(&mut self) -> bool {
        if !self.focus(self.target.window) {
            let front = unsafe { GetForegroundWindow() };
            eprintln!(
                "the target text box would not come to the foreground; aborting. In front: {front:?} {:?} (pid {})",
                window_title(front),
                pid_of(front)
            );
            return false;
        }
        println!("target: {TARGET_TITLE} (edit control {:?})\n", self.target.edit);
        true
    }

    /// Brings a window forward through the same handoff an insertion uses, and
    /// says whether it is genuinely the foreground window afterwards. Nothing
    /// may be typed into or read from a window without this answering true.
    fn focus(&mut self, window: HWND) -> bool {
        platform::remember_previous_foreground(window as isize);
        if let Err(error) = self.handoff.yield_to_previous() {
            eprintln!("          (handoff refused: {error})");
            return false;
        }
        unsafe { GetForegroundWindow() == window }
    }

    /// One throwaway insertion before anything is graded. The first paste after
    /// the target has just opened loses a cold-start race intermittently; a
    /// warm-up pays that cost where nothing is measured.
    fn warm_up(&mut self) {
        if self.clear() {
            let _ = self.exchange.insert("warm-up", None);
            self.settle();
        }
        self.target.set_text("");
    }

    /// Empties the target and leaves it in front with its edit focused.
    fn clear(&mut self) -> bool {
        self.target.set_text("");
        std::thread::sleep(Duration::from_millis(100));
        self.focus(self.target.window)
    }

    fn set_user_clipboard(&mut self) {
        let _ = self.clipboard.set_text(USER_CLIPBOARD.to_string());
        std::thread::sleep(Duration::from_millis(100));
    }

    fn clipboard_text(&self) -> String {
        self.clipboard.get_text().unwrap_or_default()
    }

    fn settle(&self) {
        std::thread::sleep(Duration::from_millis(300));
    }

    fn text_reaches_the_application(&mut self) {
        println!("-- text reaches the previous window");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.set_user_clipboard();

        if let Err(error) = self.exchange.insert("Best,\r\nNico", None) {
            self.check("insertion succeeds", false, format!("{error}"));
            return;
        }
        self.settle();
        let landed = self.target.text();
        self.check(
            "the text arrives in the edit control",
            landed.contains("Best,") && landed.contains("Nico"),
            format!("the control holds {}", Self::brief(&landed)),
        );
    }

    fn the_users_clipboard_survives(&mut self) {
        println!("-- the user's clipboard survives an insertion");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.set_user_clipboard();

        let _ = self.exchange.insert("borrowed", None);
        self.settle();

        let after = self.clipboard_text();
        let landed = self.target.text();
        let inserted = landed.contains("borrowed");
        self.check(
            "the clipboard holds what the user had",
            inserted && after == USER_CLIPBOARD,
            if inserted {
                format!("the clipboard holds {after:?}, expected {USER_CLIPBOARD:?}")
            } else {
                format!("nothing was inserted: {}", Self::brief(&landed))
            },
        );
    }

    fn an_image_on_the_clipboard_survives(&mut self) {
        println!("-- an image on the clipboard survives an insertion");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }

        let png = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../static/favicon.png"
        ))
        .expect("the repo's favicon");
        let Ok(image) = clipboard_rs::RustImageData::from_bytes(&png) else {
            self.check("an image can be put on the clipboard", false, "decode failed".into());
            return;
        };
        if self.clipboard.set_image(image).is_err() {
            self.check("an image can be put on the clipboard", false, "set failed".into());
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
        let before = self.clipboard_png_len();

        let _ = self.exchange.insert("over an image", None);
        self.settle();
        let after = self.clipboard_png_len();

        self.check(
            "the image is still on the clipboard afterwards",
            after.is_some() && after == before,
            format!("held {before:?} bytes of PNG before, {after:?} after"),
        );
        let landed = self.target.text();
        self.check(
            "and the text still arrived",
            landed.contains("over an image"),
            format!("the control holds {}", Self::brief(&landed)),
        );
    }

    fn clipboard_png_len(&self) -> Option<usize> {
        self.clipboard
            .get_image()
            .ok()
            .and_then(|image| image.to_png().ok())
            .map(|png| png.get_bytes().len())
    }

    fn both_writes_are_declared(&mut self) {
        println!("-- every borrowed write is declared to the history");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.set_user_clipboard();

        let before = self.declared.count();
        let _ = self.exchange.insert("declared", None);
        self.settle();
        let declared = self.declared.count() - before;

        self.check(
            "the insertion declares its writes",
            declared == 2,
            format!("declared {declared} writes, expected 2: the text out and the restore"),
        );
    }

    fn the_caret_lands_where_asked(&mut self) {
        println!("-- the caret lands where the template asked");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.set_user_clipboard();

        let _ = self.exchange.insert("<b></b>", Some(3));
        self.settle();

        if unsafe { GetForegroundWindow() } != self.target.window {
            self.check(
                "the target is still in front after the insertion",
                false,
                "another window took the foreground".into(),
            );
            return;
        }
        let _ = self.enigo.text("HERE");
        self.settle();

        let landed = self.target.text();
        self.check(
            "typing after the insertion lands at the caret",
            landed.contains("<b>HERE</b>"),
            format!("the control holds {}, expected <b>HERE</b>", Self::brief(&landed)),
        );
    }

    fn the_selection_is_read(&mut self) {
        println!("-- the selection is read through the clipboard round trip");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.target.set_text("quoted material");
        self.set_user_clipboard();
        self.target.select_all();
        std::thread::sleep(Duration::from_millis(100));

        let read = self.exchange.selection();
        self.settle();

        match read {
            Ok(Some(text)) => {
                self.check(
                    "the selection comes back",
                    text.contains("quoted material"),
                    format!("read {}", Self::brief(&text)),
                );
                let after = self.clipboard_text();
                self.check(
                    "reading the selection gives the clipboard back",
                    after == USER_CLIPBOARD,
                    format!("the clipboard holds {after:?}"),
                );
            }
            Ok(None) => self.check(
                "the selection comes back",
                false,
                "reported no selection, but the control was selected".into(),
            ),
            Err(error) => self.check("the selection comes back", false, format!("{error}")),
        }
    }

    fn an_empty_selection_is_reported_as_empty(&mut self) {
        println!("-- nothing selected is reported as nothing");
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.set_user_clipboard();

        let start = Instant::now();
        let read = self.exchange.selection();
        let elapsed = start.elapsed();
        self.settle();

        self.check(
            "an empty control reports no selection",
            matches!(read, Ok(None)),
            format!("expected Ok(None), got {read:?}"),
        );
        let after = self.clipboard_text();
        self.check(
            "and leaves the clipboard alone",
            after == USER_CLIPBOARD,
            format!("the clipboard holds {after:?}"),
        );
        println!("          (an empty read took {elapsed:?}, which is the copy timeout)");
    }

    /// The spec gives text 400ms to appear. The insertion call also carries the
    /// clipboard restore, so the moment the text is visible is measured
    /// separately from the call returning.
    fn text_appears_promptly(&mut self) {
        println!("-- the text appears promptly");
        let mut appeared = Vec::new();
        let mut returned = Vec::new();
        for _ in 0..5 {
            if !self.clear() {
                self.check("the target is drivable", false, "could not focus it".into());
                return;
            }
            self.set_user_clipboard();

            let exchange = self.exchange.clone();
            let start = Instant::now();
            let insertion = std::thread::spawn(move || {
                let result = exchange.insert("prompt", None);
                (result, start.elapsed())
            });
            let visible_by = Instant::now() + Duration::from_secs(2);
            let mut visible = None;
            while Instant::now() < visible_by {
                if self.target.text().contains("prompt") {
                    visible = Some(start.elapsed());
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            let (result, took) = insertion.join().unwrap();
            if result.is_err() {
                self.check("insertion succeeds", false, format!("{result:?}"));
                return;
            }
            appeared.push(visible);
            returned.push(took);
            self.settle();
        }
        let worst = appeared.iter().map(|v| v.unwrap_or(Duration::MAX)).max().unwrap();
        self.check(
            "the text is visible within 400ms on every run",
            worst < Duration::from_millis(400),
            format!("visible after {appeared:?}"),
        );
        println!("          (visible after {appeared:?})");
        println!("          (insert returned after {returned:?}, restore included)");
    }

    /// The launcher has the foreground when an action fires, so the exchange
    /// has to take it back before typing. Another window standing in for the
    /// launcher makes that visible: the text has to land in the target and not
    /// in whatever was in front.
    fn the_target_is_brought_forward_first(&mut self) {
        println!("-- the previous window is brought forward before anything is sent");
        let Some(decoy) = TextBox::open(DECOY_TITLE, 700) else {
            self.check("a decoy window opens", false, "could not open it".into());
            return;
        };
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            decoy.close();
            return;
        }
        if !self.focus(decoy.window) {
            self.check("the decoy takes the foreground", false, "could not focus it".into());
            decoy.close();
            return;
        }
        self.set_user_clipboard();
        platform::remember_previous_foreground(self.target.window as isize);

        let result = self.exchange.insert("handed off", None);
        self.settle();

        let in_front = unsafe { GetForegroundWindow() };
        let target_text = self.target.text();
        let decoy_text = decoy.text();
        self.check(
            "the target is the foreground window afterwards",
            in_front == self.target.window,
            format!("foreground is {in_front:?}, target is {:?}", self.target.window),
        );
        self.check(
            "the text landed in the target and not in the window that was in front",
            result.is_ok() && target_text.contains("handed off") && decoy_text.is_empty(),
            format!(
                "result {result:?}; target holds {}; decoy holds {}",
                Self::brief(&target_text),
                Self::brief(&decoy_text)
            ),
        );
        decoy.close();
    }

    /// A window the user closed while the launcher was up can never come back
    /// to the foreground. Nothing may be sent, and the clipboard must be left
    /// alone, since the failure happens before anything is borrowed.
    fn a_vanished_target_is_reported(&mut self) {
        println!("-- a previous window that never comes forward is reported, not skipped over");
        let Some(decoy) = TextBox::open(DECOY_TITLE, 700) else {
            self.check("a decoy window opens", false, "could not open it".into());
            return;
        };
        let stale = decoy.window;
        decoy.close();
        if !self.clear() {
            self.check("the target is drivable", false, "could not focus it".into());
            return;
        }
        self.set_user_clipboard();
        let before = self.declared.count();
        platform::remember_previous_foreground(stale as isize);

        let start = Instant::now();
        let result = self.exchange.insert("never sent", None);
        let elapsed = start.elapsed();
        self.settle();

        self.check(
            "the insertion fails with a message about the foreground",
            matches!(&result, Err(TextError::TargetUnavailable(m)) if m.contains("foreground")),
            format!("got {result:?} after {elapsed:?}"),
        );
        let landed = self.target.text();
        let after = self.clipboard_text();
        self.check(
            "nothing was sent and the clipboard is untouched",
            landed.is_empty() && after == USER_CLIPBOARD && self.declared.count() == before,
            format!(
                "target holds {}, clipboard holds {after:?}, {} writes declared",
                Self::brief(&landed),
                self.declared.count() - before
            ),
        );
        println!("          (gave up after {elapsed:?})");
    }

    fn no_target_is_reported(&mut self) {
        println!("-- no previous window at all is reported");
        self.set_user_clipboard();
        platform::remember_previous_foreground(0);
        let result = self.exchange.insert("nowhere", None);
        self.check(
            "the insertion fails with NoTarget",
            matches!(result, Err(TextError::NoTarget)),
            format!("got {result:?}"),
        );
        let after = self.clipboard_text();
        self.check(
            "and the clipboard is untouched",
            after == USER_CLIPBOARD,
            format!("the clipboard holds {after:?}"),
        );
    }

    /// Task 1.5: does a no-op message answered by the target mean the paste
    /// has been handled? The harness pastes a marker itself, waits for the
    /// acknowledgement, and at once overwrites the clipboard with a decoy. What
    /// the control holds says which of the two the paste read.
    fn the_paste_acknowledgement_is_measured(&mut self) {
        println!("-- the no-op acknowledgement after Ctrl+V");
        let mut marker_won = 0;
        let mut decoy_won = 0;
        let mut nothing = 0;
        let mut acks = Vec::new();
        for trial in 0..10 {
            if !self.clear() {
                self.check("the target is drivable", false, "could not focus it".into());
                return;
            }
            let marker = format!("marker-{trial}");
            let decoy = format!("decoy-{trial}");
            let _ = self.clipboard.set_text(marker.clone());
            std::thread::sleep(Duration::from_millis(100));

            let _ = self.enigo.key(Key::Control, Direction::Press);
            let _ = self.enigo.key(Key::Unicode('v'), Direction::Click);
            let _ = self.enigo.key(Key::Control, Direction::Release);
            let start = Instant::now();
            self.handoff.settle_after_paste();
            acks.push(start.elapsed());
            let _ = self.clipboard.set_text(decoy.clone());

            std::thread::sleep(Duration::from_millis(400));
            let landed = self.target.text();
            if landed.contains(&marker) {
                marker_won += 1;
            } else if landed.contains(&decoy) {
                decoy_won += 1;
            } else {
                nothing += 1;
            }
        }
        self.check(
            "the paste had been handled every time the acknowledgement came back",
            marker_won == 10,
            format!("marker {marker_won}, decoy {decoy_won}, nothing {nothing}"),
        );
        println!("          (marker {marker_won}, decoy {decoy_won}, nothing {nothing}; acknowledgement took {acks:?})");
    }

    fn an_elevated_target_is_refused(&mut self, integrity: Option<u32>) {
        println!("-- an elevated target is refused rather than pasted into silently");
        if integrity.is_some_and(|level| level > MEDIUM_INTEGRITY) {
            self.skip("an elevated target is refused", "this harness is elevated itself".into());
            return;
        }
        let Some(elevated) = find_window(|title| title.contains(ELEVATED_TITLE)) else {
            self.skip(
                "an elevated target is refused",
                format!("no window titled *{ELEVATED_TITLE}*; open one elevated first"),
            );
            return;
        };
        self.set_user_clipboard();
        let before = self.declared.count();
        platform::remember_previous_foreground(elevated as isize);

        let result = self.exchange.insert("into an elevated window", None);
        self.settle();

        self.check(
            "the insertion fails with a message about elevation",
            matches!(&result, Err(TextError::TargetUnavailable(m)) if m.contains("elevated")),
            format!("got {result:?}"),
        );
        let after = self.clipboard_text();
        self.check(
            "and the clipboard is untouched",
            after == USER_CLIPBOARD && self.declared.count() == before,
            format!(
                "the clipboard holds {after:?}, {} writes declared",
                self.declared.count() - before
            ),
        );
        let _ = self.focus(self.target.window);
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

fn dango_is_running() -> bool {
    Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq dango.exe", "/FO", "CSV", "/NH"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("dango.exe"))
        .unwrap_or(false)
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> i32 {
    let windows = &mut *(lparam as *mut Vec<HWND>);
    windows.push(hwnd);
    TRUE
}

fn top_level_windows() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(Some(collect), &mut windows as *mut Vec<HWND> as LPARAM);
    }
    windows
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

fn pid_of(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    pid
}

fn find_window(matches: impl Fn(&str) -> bool) -> Option<HWND> {
    top_level_windows()
        .into_iter()
        .find(|&hwnd| matches(&window_title(hwnd)))
}

fn wait_for_window(matches: impl Fn(&str) -> bool, timeout: Duration) -> Option<HWND> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(hwnd) = find_window(&matches) {
            return Some(hwnd);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `WM_GETTEXT` is marshalled across processes by the system, so an edit
/// control in another application answers with its contents directly.
fn text_of(hwnd: HWND) -> String {
    unsafe {
        let len = SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0).max(0) as usize;
        let mut buffer = vec![0u16; len + 1];
        let copied = SendMessageW(
            hwnd,
            WM_GETTEXT,
            buffer.len(),
            buffer.as_mut_ptr() as LPARAM,
        )
        .max(0) as usize;
        String::from_utf16_lossy(&buffer[..copied])
    }
}
