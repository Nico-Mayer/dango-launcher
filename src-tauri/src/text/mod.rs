//! Reading what the user has selected, and putting text back where they were.
//!
//! One path, used by everything that ever needs one. The interesting part is
//! that both directions go through the clipboard, and the user must never be
//! able to tell: their content is saved, borrowed, and put back, and none of it
//! reaches the clipboard history.
//!
//! The platforms differ in three narrow places, which is what the traits below
//! are for: which keys mean copy and paste, how the launcher hands focus back,
//! and whether the selection can be read without touching the clipboard at all.

use std::sync::Arc;

use crate::extensions::clipboard::{ClipboardSource, Content};

/// How long to wait for a copy to land before calling the selection empty.
/// Inside the 300ms the spec gives a selection read, with room for the
/// keystroke that precedes it.
const COPY_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(200);
const COPY_POLL: std::time::Duration = std::time::Duration::from_millis(5);
/// How long a paste is given to be consumed before the clipboard is restored.
///
/// `settle_after_paste` confirms the target thread is responsive, but a sent
/// message is handled before the posted paste keystroke is consumed, and a
/// Chromium-based application then hands the paste to a renderer process that
/// reads the clipboard later still. Without the grace the restore wins that
/// race and the user's own clipboard is pasted instead of the text. Still far
/// inside the 400ms an insertion and the 500ms an expansion are allowed.
const PASTE_GRACE: std::time::Duration = std::time::Duration::from_millis(120);

pub const KEYSTROKE_FAILED: &str = "Couldn't send the keystroke. Try again.";

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum TextError {
    #[error("Dango needs the Accessibility permission to paste. Grant it in System Settings, Privacy & Security.")]
    PermissionMissing,
    #[error("Nothing to paste into. Switch to an app first.")]
    NoTarget,
    #[error("{0}")]
    TargetUnavailable(String),
    #[error("Couldn't use the clipboard. Try again.")]
    Clipboard,
}

/// The keystrokes, per platform: `Cmd` on macOS, `Ctrl` on Windows, and the
/// layout mapping underneath that neither of them should own.
pub trait Keys: Send + Sync {
    fn copy(&self) -> Result<(), TextError>;
    fn paste(&self) -> Result<(), TextError>;
    fn caret_left(&self, times: usize) -> Result<(), TextError>;
    /// Deletes the character before the caret `times` times, for removing a
    /// typed keyword before its expansion is pasted.
    fn backspace(&self, times: usize) -> Result<(), TextError>;
    /// macOS gates all of this behind Accessibility. Windows has no equivalent
    /// and always answers true.
    fn permitted(&self) -> bool;
    fn request_permission(&self);
}

/// Getting the launcher out of the way and handing focus back. The two
/// platforms are not the same operation wearing different names: Windows has to
/// take the foreground back and wait for it, while macOS never took application
/// activation at all and only has to stop being the key window.
pub trait Handoff: Send + Sync {
    /// Returns once the previous application is genuinely ready to receive
    /// input, or reports why it never will be.
    fn yield_to_previous(&self) -> Result<(), TextError>;
    /// Called after the paste keystroke and before the clipboard is restored.
    /// Windows asks the target to answer a no-op message, which it only does
    /// once it has handled the paste. macOS has no such signal and waits.
    ///
    /// Only paste needs this. A copy leaves an observable result, so the
    /// exchange watches for it instead of being told how long to wait.
    fn settle_after_paste(&self);
}

/// Reading the selection without the clipboard, where the platform can. macOS
/// asks the accessibility API; Windows has no implementation and falls through
/// to the clipboard round trip.
pub trait DirectSelection: Send + Sync {
    fn selected_text(&self) -> Option<String>;
}

/// Declares a write Dango is about to make so the clipboard history ignores it.
/// A trait rather than the watcher itself, because the exchange has to work
/// when the clipboard extension is disabled.
pub trait OwnWrites: Send + Sync {
    fn expect(&self, content: &Content);
}

/// Runs a closure on the process's main thread and waits for the result.
///
/// Not a convenience. Key synthesis on macOS reaches the Text Services Manager
/// to map a character to a keycode, and that asserts it is on the main queue:
/// off it, the process traps. The orchestration around the keystrokes must stay
/// off the main thread, because it sleeps and the hide it waits for needs the
/// main run loop, so only the keystrokes themselves hop across.
pub trait MainThread: Send + Sync {
    fn run(&self, work: Box<dyn FnOnce() + Send>);
}

/// For platforms with no such rule, and for tests. Runs it where it stands.
pub struct HereIsFine;

impl MainThread for HereIsFine {
    fn run(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }
}

/// Getting the launcher off screen. Inserting has to happen after the launcher
/// is gone, or the keystroke lands in Dango's own search field, so the exchange
/// owns that ordering rather than trusting a caller to hide first.
pub trait Launcher: Send + Sync {
    fn dismiss(&self);
}

/// Nothing to tell, because nothing is listening.
pub struct Unwatched;

impl OwnWrites for Unwatched {
    fn expect(&self, _content: &Content) {}
}

/// What a consumer of this actually needs. A trait so an extension depends on
/// three methods rather than on the whole round trip, and so it can be tested
/// without a clipboard, a keyboard, or a window.
pub trait TextTarget: Send + Sync {
    fn insert(&self, text: &str, caret: Option<usize>) -> Result<(), TextError>;
    /// Deletes `backspaces` characters before the caret, then inserts `text`.
    /// For keyword expansion, where the focus is already the target and no
    /// launcher is involved.
    fn expand(&self, backspaces: usize, text: &str, caret: Option<usize>) -> Result<(), TextError>;
    /// Puts `content` on the clipboard and pastes it, leaving it there. Unlike
    /// `insert`, nothing is restored afterwards: the content is the user's own
    /// choice of what the clipboard should hold.
    fn paste_content(&self, content: &Content) -> Result<(), TextError>;
    fn selection(&self) -> Result<Option<String>, TextError>;
    fn clipboard_text(&self) -> Option<String>;
}

impl TextTarget for TextExchange {
    fn insert(&self, text: &str, caret: Option<usize>) -> Result<(), TextError> {
        TextExchange::insert(self, text, caret)
    }

    fn paste_content(&self, content: &Content) -> Result<(), TextError> {
        TextExchange::paste_content(self, content)
    }

    fn expand(&self, backspaces: usize, text: &str, caret: Option<usize>) -> Result<(), TextError> {
        TextExchange::expand(self, backspaces, text, caret)
    }

    fn selection(&self) -> Result<Option<String>, TextError> {
        TextExchange::selection(self)
    }

    fn clipboard_text(&self) -> Option<String> {
        TextExchange::clipboard_text(self)
    }
}

/// The shared round trip. Both directions borrow the clipboard, so both live
/// here rather than in either platform.
pub struct TextExchange {
    clipboard: Arc<dyn ClipboardSource>,
    keys: Arc<dyn Keys>,
    handoff: Arc<dyn Handoff>,
    direct: Option<Arc<dyn DirectSelection>>,
    own_writes: Arc<dyn OwnWrites>,
    launcher: Arc<dyn Launcher>,
}

impl TextExchange {
    pub fn new(
        clipboard: Arc<dyn ClipboardSource>,
        keys: Arc<dyn Keys>,
        handoff: Arc<dyn Handoff>,
        direct: Option<Arc<dyn DirectSelection>>,
        own_writes: Arc<dyn OwnWrites>,
        launcher: Arc<dyn Launcher>,
    ) -> Self {
        Self {
            clipboard,
            keys,
            handoff,
            direct,
            own_writes,
            launcher,
        }
    }

    pub fn permitted(&self) -> bool {
        self.keys.permitted()
    }

    pub fn request_permission(&self) {
        self.keys.request_permission();
    }

    /// The clipboard's text, for a template that asks for it. Reading is free
    /// and touches nothing, unlike the selection.
    pub fn clipboard_text(&self) -> Option<String> {
        self.clipboard.text().filter(|text| !text.is_empty())
    }

    /// What the user has selected in the frontmost application. `None` means
    /// there is no selection; an error means it could not be asked.
    ///
    /// The accessibility route is tried first where there is one, because it
    /// touches nothing. The clipboard round trip is the fallback on both
    /// platforms, so the tricky path is the one that gets exercised everywhere.
    pub fn selection(&self) -> Result<Option<String>, TextError> {
        if !self.keys.permitted() {
            return Err(TextError::PermissionMissing);
        }
        if let Some(direct) = &self.direct {
            if let Some(text) = direct.selected_text() {
                return Ok(Some(text).filter(|text| !text.is_empty()));
            }
        }
        self.selection_through_clipboard()
    }

    /// Copying with nothing selected leaves the clipboard exactly as it was, so
    /// reading it back cannot tell "nothing was selected" from "the selection
    /// happened to be what was already there". A sentinel written before the
    /// copy removes the ambiguity: if it survives, the copy did nothing.
    fn selection_through_clipboard(&self) -> Result<Option<String>, TextError> {
        let saved = self.read_clipboard();

        self.launcher.dismiss();
        self.handoff.yield_to_previous()?;

        let sentinel = format!("dango-selection-{}", uuid::Uuid::new_v4());
        self.own_writes.expect(&Content::Text(sentinel.clone()));
        self.clipboard.set_text(&sentinel);

        let after_copy = self.keys.copy().map(|()| self.wait_for_copy(&sentinel))?;

        // One read, not two: anything that appears after this point came from
        // the user, and re-reading would hand them their own content back as
        // the thing Dango borrowed.
        let borrowed = after_copy.clone().unwrap_or_else(|| sentinel.clone());
        self.restore(saved, &borrowed);

        Ok(after_copy.filter(|text| *text != sentinel && !text.is_empty()))
    }

    /// Unlike a paste, a copy leaves something observable: the clipboard stops
    /// being the sentinel. So this waits for the answer rather than guessing at
    /// a delay, and returns the moment it arrives.
    ///
    /// A copy that was never going to produce anything, because nothing was
    /// selected, costs the whole timeout. That is the price of telling an empty
    /// selection from a slow one.
    fn wait_for_copy(&self, sentinel: &str) -> Option<String> {
        let deadline = std::time::Instant::now() + COPY_TIMEOUT;
        loop {
            let current = self.clipboard.text();
            match current {
                Some(text) if text != sentinel => return Some(text),
                _ if std::time::Instant::now() >= deadline => return None,
                _ => std::thread::sleep(COPY_POLL),
            }
        }
    }

    /// Puts text into the application the user was in. The caret, when given,
    /// is a character offset into `text` where it should end up.
    pub fn insert(&self, text: &str, caret: Option<usize>) -> Result<(), TextError> {
        if !self.keys.permitted() {
            return Err(TextError::PermissionMissing);
        }

        let saved = self.read_clipboard();
        // Before anything is sent. A keystroke goes wherever the focus is, and
        // until this returns the focus is still Dango's own search field.
        self.launcher.dismiss();
        self.handoff.yield_to_previous()?;

        let outgoing = Content::Text(text.to_string());
        self.own_writes.expect(&outgoing);
        self.clipboard.set_text(text);

        // A failed paste must leave the clipboard as it was found, so the
        // restore happens either way.
        let pasted = self.keys.paste();
        self.handoff.settle_after_paste();
        if pasted.is_ok() {
            self.place_caret(text, caret)?;
            std::thread::sleep(PASTE_GRACE);
        }
        self.restore(saved, text);
        pasted
    }

    /// Pastes a clipboard history entry. The order is what makes the failure
    /// cases clean: a missing permission or a missing target returns before
    /// the clipboard is written, so only a paste that fails after the write
    /// leaves the entry on the clipboard, where the user can still paste it.
    pub fn paste_content(&self, content: &Content) -> Result<(), TextError> {
        if !self.keys.permitted() {
            return Err(TextError::PermissionMissing);
        }

        self.launcher.dismiss();
        self.handoff.yield_to_previous()?;

        self.own_writes.expect(content);
        self.write(content);

        let pasted = self.keys.paste();
        self.handoff.settle_after_paste();
        pasted
    }

    /// Replaces a just-typed keyword with `text`: deletes the keyword with
    /// `backspaces` backspaces, then pastes the text and restores the clipboard.
    ///
    /// Unlike `insert`, it neither dismisses a launcher nor yields the
    /// foreground: keyword expansion happens in the application the user is
    /// already typing in, so the focus is right and must not be disturbed. The
    /// backspaces run before the paste, so a keyword that did not match what the
    /// application held shows as visible text rather than a silent corruption.
    pub fn expand(
        &self,
        backspaces: usize,
        text: &str,
        caret: Option<usize>,
    ) -> Result<(), TextError> {
        if !self.keys.permitted() {
            return Err(TextError::PermissionMissing);
        }

        let saved = self.read_clipboard();
        // The target is already the foreground here, unlike the launcher case, so
        // this is a re-assert rather than a focus change. It is still needed: it
        // is what makes the paste land before the clipboard is restored, the same
        // ordering the launcher insert relies on. Without it the restore can race
        // the paste and the user's own clipboard is pasted instead.
        self.handoff.yield_to_previous()?;

        if backspaces > 0 {
            self.keys.backspace(backspaces)?;
        }

        let outgoing = Content::Text(text.to_string());
        self.own_writes.expect(&outgoing);
        self.clipboard.set_text(text);

        let pasted = self.keys.paste();
        self.handoff.settle_after_paste();
        if pasted.is_ok() {
            self.place_caret(text, caret)?;
            std::thread::sleep(PASTE_GRACE);
        }
        self.restore(saved, text);
        pasted
    }

    fn place_caret(&self, text: &str, caret: Option<usize>) -> Result<(), TextError> {
        let Some(caret) = caret else {
            return Ok(());
        };
        // Characters, not bytes: an arrow key moves by neither bytes nor
        // graphemes, and counting bytes puts the caret in the wrong place in
        // anything that is not ASCII.
        let after = text.chars().count().saturating_sub(caret);
        if after == 0 {
            return Ok(());
        }
        self.keys.caret_left(after)
    }

    fn read_clipboard(&self) -> Option<Content> {
        if let Some(text) = self.clipboard.text().filter(|text| !text.is_empty()) {
            return Some(Content::Text(text));
        }
        self.clipboard.image().map(Content::Image)
    }

    /// Puts back what the user had, unless they have copied something else in
    /// the meantime. Their newer content wins: they have moved on, and putting
    /// the old clipboard back over it would be the one failure they would
    /// actually notice.
    ///
    /// `borrowed` is what Dango last wrote. Anything else on the clipboard now
    /// came from the user.
    fn restore(&self, saved: Option<Content>, borrowed: &str) {
        let Some(saved) = saved else {
            return;
        };
        if self.clipboard.text().as_deref() != Some(borrowed) {
            return;
        }
        self.own_writes.expect(&saved);
        self.write(&saved);
    }

    fn write(&self, content: &Content) {
        match content {
            Content::Text(text) => self.clipboard.set_text(text),
            Content::Image(png) => self.clipboard.set_image(png),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeClipboard {
        content: Mutex<Option<Content>>,
        writes: Mutex<Vec<Content>>,
        /// The user copying something of their own the moment after Dango has
        /// looked, which is the window a restore has to respect.
        after_next_read: Mutex<Option<String>>,
    }

    impl FakeClipboard {
        fn holding(content: Content) -> Arc<Self> {
            let clipboard = Arc::new(Self::default());
            *clipboard.content.lock().unwrap() = Some(content);
            clipboard
        }

        fn now(&self) -> Option<Content> {
            self.content.lock().unwrap().clone()
        }
    }

    impl ClipboardSource for FakeClipboard {
        fn formats(&self) -> Vec<String> {
            vec![]
        }
        fn text(&self) -> Option<String> {
            let read = match self.content.lock().unwrap().clone() {
                Some(Content::Text(text)) => Some(text),
                _ => None,
            };
            if let Some(newer) = self.after_next_read.lock().unwrap().take() {
                *self.content.lock().unwrap() = Some(Content::Text(newer));
            }
            read
        }
        fn image(&self) -> Option<Vec<u8>> {
            match self.content.lock().unwrap().clone() {
                Some(Content::Image(png)) => Some(png),
                _ => None,
            }
        }
        fn set_text(&self, text: &str) {
            let content = Content::Text(text.to_string());
            self.writes.lock().unwrap().push(content.clone());
            *self.content.lock().unwrap() = Some(content);
        }
        fn set_image(&self, png: &[u8]) {
            let content = Content::Image(png.to_vec());
            self.writes.lock().unwrap().push(content.clone());
            *self.content.lock().unwrap() = Some(content);
        }
    }

    #[derive(Default)]
    struct FakeKeys {
        permitted: bool,
        paste_fails: bool,
        events: Mutex<Vec<String>>,
        /// What the target "types" when it is pasted into.
        pastes_into: Option<Arc<FakeClipboard>>,
        /// The user copying something of their own while Dango is mid-operation.
        copies_during_paste: Option<(Arc<FakeClipboard>, String)>,
        /// The same, but landing just after Dango reads what it borrowed.
        copies_after_read: Option<(Arc<FakeClipboard>, String)>,
    }

    impl FakeKeys {
        fn working() -> Arc<Self> {
            Arc::new(Self {
                permitted: true,
                ..Default::default()
            })
        }
        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }

        fn interrupt(&self) {
            if let Some((clipboard, text)) = &self.copies_during_paste {
                clipboard.set_text(text);
            }
            if let Some((clipboard, text)) = &self.copies_after_read {
                *clipboard.after_next_read.lock().unwrap() = Some(text.clone());
            }
        }
    }

    impl Keys for FakeKeys {
        fn copy(&self) -> Result<(), TextError> {
            self.events.lock().unwrap().push("copy".into());
            if let Some(clipboard) = &self.pastes_into {
                clipboard.set_text("what was selected");
                clipboard.writes.lock().unwrap().pop();
            }
            self.interrupt();
            Ok(())
        }
        fn paste(&self) -> Result<(), TextError> {
            self.events.lock().unwrap().push("paste".into());
            if self.paste_fails {
                return Err(TextError::TargetUnavailable("nope".into()));
            }
            self.interrupt();
            Ok(())
        }
        fn caret_left(&self, times: usize) -> Result<(), TextError> {
            self.events.lock().unwrap().push(format!("left {times}"));
            Ok(())
        }
        fn backspace(&self, times: usize) -> Result<(), TextError> {
            self.events
                .lock()
                .unwrap()
                .push(format!("backspace {times}"));
            Ok(())
        }
        fn permitted(&self) -> bool {
            self.permitted
        }
        fn request_permission(&self) {}
    }

    #[derive(Default)]
    struct FakeHandoff {
        fails: Option<TextError>,
        yielded: Mutex<usize>,
    }

    impl FakeHandoff {
        fn working() -> Arc<Self> {
            Arc::new(Self::default())
        }
    }

    impl Handoff for FakeHandoff {
        fn yield_to_previous(&self) -> Result<(), TextError> {
            *self.yielded.lock().unwrap() += 1;
            match &self.fails {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }
        fn settle_after_paste(&self) {}
    }

    #[derive(Default)]
    struct FakeLauncher(Mutex<usize>);

    impl FakeLauncher {
        fn dismissals(&self) -> usize {
            *self.0.lock().unwrap()
        }
    }

    impl Launcher for FakeLauncher {
        fn dismiss(&self) {
            *self.0.lock().unwrap() += 1;
        }
    }

    #[derive(Default)]
    struct RecordingWrites(Mutex<Vec<Content>>);

    impl RecordingWrites {
        fn declared(&self) -> Vec<Content> {
            self.0.lock().unwrap().clone()
        }
    }

    impl OwnWrites for RecordingWrites {
        fn expect(&self, content: &Content) {
            self.0.lock().unwrap().push(content.clone());
        }
    }

    struct Fixture {
        exchange: TextExchange,
        clipboard: Arc<FakeClipboard>,
        keys: Arc<FakeKeys>,
        handoff: Arc<FakeHandoff>,
        writes: Arc<RecordingWrites>,
        launcher: Arc<FakeLauncher>,
    }

    fn fixture(clipboard: Arc<FakeClipboard>, keys: Arc<FakeKeys>) -> Fixture {
        let handoff = FakeHandoff::working();
        let writes = Arc::new(RecordingWrites::default());
        let launcher = Arc::new(FakeLauncher::default());
        Fixture {
            exchange: TextExchange::new(
                clipboard.clone(),
                keys.clone(),
                handoff.clone(),
                None,
                writes.clone(),
                launcher.clone(),
            ),
            clipboard,
            keys,
            handoff,
            writes,
            launcher,
        }
    }

    fn holding(text: &str) -> Arc<FakeClipboard> {
        FakeClipboard::holding(Content::Text(text.into()))
    }

    #[test]
    fn expanding_backspaces_before_it_pastes_and_leaves_no_launcher_dismiss() {
        let f = fixture(holding("what the user had"), FakeKeys::working());

        f.exchange.expand(4, "the snippet", None).unwrap();

        // The keyword is deleted before the paste replaces it. Expansion never
        // dismisses a launcher, but it does re-assert the foreground so the paste
        // lands before the clipboard is restored.
        assert_eq!(f.keys.events(), vec!["backspace 4", "paste"]);
        assert_eq!(f.launcher.dismissals(), 0, "expansion has no launcher");
        assert_eq!(*f.handoff.yielded.lock().unwrap(), 1);
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into())),
            "the user's clipboard is theirs"
        );
    }

    #[test]
    fn inserting_text_pastes_it_and_puts_the_clipboard_back() {
        let f = fixture(holding("what the user had"), FakeKeys::working());

        f.exchange.insert("the snippet", None).unwrap();

        assert_eq!(f.keys.events(), vec!["paste"]);
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into())),
            "the user's clipboard is theirs"
        );
    }

    #[test]
    fn the_launcher_gets_out_of_the_way_before_anything_is_sent() {
        let f = fixture(holding("saved"), FakeKeys::working());
        f.exchange.insert("text", None).unwrap();
        assert_eq!(*f.handoff.yielded.lock().unwrap(), 1);
        assert_eq!(
            f.launcher.dismissals(),
            1,
            "a keystroke sent while the launcher is up lands in its own search field"
        );
    }

    #[test]
    fn reading_a_selection_also_dismisses_first() {
        let clipboard = holding("what the user had");
        let keys = Arc::new(FakeKeys {
            permitted: true,
            pastes_into: Some(clipboard.clone()),
            ..Default::default()
        });
        let f = fixture(clipboard, keys);

        f.exchange.selection().unwrap();

        assert_eq!(f.launcher.dismissals(), 1, "the copy is a keystroke too");
    }

    #[test]
    fn nothing_is_dismissed_when_the_permission_is_missing() {
        let f = fixture(holding("saved"), Arc::new(FakeKeys::default()));
        assert!(f.exchange.insert("text", None).is_err());
        assert_eq!(
            f.launcher.dismissals(),
            0,
            "hiding for an insertion that cannot happen loses the message"
        );
    }

    #[test]
    fn an_image_on_the_clipboard_survives_an_insertion() {
        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
        let clipboard = FakeClipboard::holding(Content::Image(png.clone()));
        let f = fixture(clipboard, FakeKeys::working());

        f.exchange.insert("the snippet", None).unwrap();

        assert_eq!(f.clipboard.now(), Some(Content::Image(png)));
    }

    #[test]
    fn both_writes_of_an_insertion_are_declared_to_the_history() {
        let f = fixture(holding("what the user had"), FakeKeys::working());
        f.exchange.insert("the snippet", None).unwrap();
        assert_eq!(
            f.writes.declared(),
            vec![
                Content::Text("the snippet".into()),
                Content::Text("what the user had".into())
            ],
            "a paste makes two writes and neither is the user's"
        );
    }

    #[test]
    fn an_empty_clipboard_leaves_nothing_to_restore() {
        let f = fixture(Arc::new(FakeClipboard::default()), FakeKeys::working());
        f.exchange.insert("the snippet", None).unwrap();
        assert_eq!(
            f.writes.declared(),
            vec![Content::Text("the snippet".into())]
        );
    }

    #[test]
    fn the_restore_is_abandoned_when_the_user_copies_during_an_insertion() {
        let clipboard = holding("what the user had");
        let keys = Arc::new(FakeKeys {
            permitted: true,
            copies_during_paste: Some((clipboard.clone(), "something newer".into())),
            ..Default::default()
        });
        let f = fixture(clipboard, keys);

        f.exchange.insert("the snippet", None).unwrap();

        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("something newer".into())),
            "the user has moved on; putting the old clipboard back over it is the \
             one failure they would notice"
        );
    }

    #[test]
    fn the_restore_is_abandoned_when_the_user_copies_during_a_selection_read() {
        let clipboard = holding("what the user had");
        // The user copies immediately after Dango reads what it borrowed, which
        // is the window between borrowing the clipboard and giving it back.
        let keys = Arc::new(FakeKeys {
            permitted: true,
            pastes_into: Some(clipboard.clone()),
            copies_after_read: Some((clipboard.clone(), "something newer".into())),
            ..Default::default()
        });
        let f = fixture(clipboard, keys);

        f.exchange.selection().unwrap();

        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("something newer".into())),
            "their newer copy wins over the one being put back"
        );
    }

    #[test]
    fn a_failed_insertion_reports_why_and_leaves_the_clipboard_alone() {
        let keys = Arc::new(FakeKeys {
            permitted: true,
            paste_fails: true,
            ..Default::default()
        });
        let f = fixture(holding("what the user had"), keys);

        let error = f.exchange.insert("the snippet", None).unwrap_err();

        assert!(matches!(error, TextError::TargetUnavailable(_)));
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into())),
            "a failure must not cost the user their clipboard"
        );
    }

    #[test]
    fn nothing_is_sent_without_the_permission() {
        let keys = Arc::new(FakeKeys::default());
        let f = fixture(holding("what the user had"), keys);

        assert_eq!(
            f.exchange.insert("the snippet", None),
            Err(TextError::PermissionMissing)
        );
        assert!(f.keys.events().is_empty(), "nothing reaches the other app");
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn pasting_content_leaves_it_on_the_clipboard() {
        let f = fixture(holding("what the user had"), FakeKeys::working());

        f.exchange
            .paste_content(&Content::Text("from the history".into()))
            .unwrap();

        assert_eq!(f.keys.events(), vec!["paste"]);
        assert_eq!(f.launcher.dismissals(), 1);
        assert_eq!(*f.handoff.yielded.lock().unwrap(), 1);
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("from the history".into())),
            "the entry is what the user chose to have on the clipboard"
        );
        assert_eq!(
            f.writes.declared(),
            vec![Content::Text("from the history".into())],
            "one write, declared so the history neither records nor reorders it"
        );
    }

    #[test]
    fn pasting_an_image_puts_the_image_on_the_clipboard() {
        let png = vec![0x89, b'P', b'N', b'G', 4, 5, 6];
        let f = fixture(holding("what the user had"), FakeKeys::working());

        f.exchange
            .paste_content(&Content::Image(png.clone()))
            .unwrap();

        assert_eq!(f.keys.events(), vec!["paste"]);
        assert_eq!(f.clipboard.now(), Some(Content::Image(png)));
    }

    #[test]
    fn pasting_content_without_the_permission_touches_nothing() {
        let f = fixture(holding("what the user had"), Arc::new(FakeKeys::default()));

        assert_eq!(
            f.exchange
                .paste_content(&Content::Text("from the history".into())),
            Err(TextError::PermissionMissing)
        );
        assert_eq!(f.launcher.dismissals(), 0);
        assert!(f.writes.declared().is_empty());
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn pasting_content_with_nowhere_to_go_touches_nothing() {
        let handoff = Arc::new(FakeHandoff {
            fails: Some(TextError::NoTarget),
            ..Default::default()
        });
        let clipboard = holding("what the user had");
        let writes = Arc::new(RecordingWrites::default());
        let exchange = TextExchange::new(
            clipboard.clone(),
            FakeKeys::working(),
            handoff,
            None,
            writes.clone(),
            Arc::new(FakeLauncher::default()),
        );

        assert_eq!(
            exchange.paste_content(&Content::Text("from the history".into())),
            Err(TextError::NoTarget)
        );
        assert!(writes.declared().is_empty());
        assert_eq!(
            clipboard.now(),
            Some(Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn there_is_nothing_to_insert_into() {
        let handoff = Arc::new(FakeHandoff {
            fails: Some(TextError::NoTarget),
            ..Default::default()
        });
        let clipboard = holding("what the user had");
        let exchange = TextExchange::new(
            clipboard.clone(),
            FakeKeys::working(),
            handoff,
            None,
            Arc::new(RecordingWrites::default()),
            Arc::new(FakeLauncher::default()),
        );

        assert_eq!(exchange.insert("text", None), Err(TextError::NoTarget));
        assert_eq!(
            clipboard.now(),
            Some(Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn the_caret_is_moved_back_by_characters_not_bytes() {
        let f = fixture(holding("saved"), FakeKeys::working());
        // Six characters, of which two are multi-byte, with the caret before
        // the last three.
        f.exchange.insert("däng☃er", Some(4)).unwrap();
        assert_eq!(f.keys.events(), vec!["paste".to_string(), "left 3".into()]);
    }

    #[test]
    fn a_caret_at_the_end_moves_nothing() {
        let f = fixture(holding("saved"), FakeKeys::working());
        f.exchange.insert("hello", Some(5)).unwrap();
        assert_eq!(f.keys.events(), vec!["paste"], "no wasted keystrokes");
    }

    #[test]
    fn no_caret_moves_nothing() {
        let f = fixture(holding("saved"), FakeKeys::working());
        f.exchange.insert("hello", None).unwrap();
        assert_eq!(f.keys.events(), vec!["paste"]);
    }

    #[test]
    fn reading_a_selection_copies_it_and_puts_the_clipboard_back() {
        let clipboard = holding("what the user had");
        let keys = Arc::new(FakeKeys {
            permitted: true,
            pastes_into: Some(clipboard.clone()),
            ..Default::default()
        });
        let f = fixture(clipboard, keys);

        let selection = f.exchange.selection().unwrap();

        assert_eq!(selection.as_deref(), Some("what was selected"));
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into())),
            "reading the selection is not allowed to cost the clipboard"
        );
        assert_eq!(f.keys.events(), vec!["copy"]);
    }

    #[test]
    fn the_sentinel_is_never_left_on_the_clipboard() {
        let clipboard = holding("what the user had");
        let keys = FakeKeys::working();
        let exchange = TextExchange::new(
            clipboard.clone(),
            keys,
            FakeHandoff::working(),
            None,
            Arc::new(RecordingWrites::default()),
            Arc::new(FakeLauncher::default()),
        );

        exchange.selection().unwrap();

        assert_eq!(
            clipboard.now(),
            Some(Content::Text("what the user had".into())),
            "a sentinel the user could see would be the worst of both"
        );
    }

    #[test]
    fn a_selection_read_declares_its_restore_to_the_history() {
        let clipboard = holding("what the user had");
        let keys = Arc::new(FakeKeys {
            permitted: true,
            pastes_into: Some(clipboard.clone()),
            ..Default::default()
        });
        let f = fixture(clipboard, keys);

        f.exchange.selection().unwrap();

        let declared = f.writes.declared();
        assert_eq!(declared.len(), 2, "the sentinel and the restore");
        assert_eq!(
            declared.last(),
            Some(&Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn nothing_selected_leaves_the_clipboard_untouched() {
        let f = fixture(holding("what the user had"), FakeKeys::working());

        // The copy changes nothing, because there was nothing selected, so the
        // sentinel is still there when the clipboard is read back.
        assert_eq!(f.exchange.selection().unwrap(), None);
        assert_eq!(
            f.clipboard.now(),
            Some(Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn a_selection_is_not_read_without_the_permission() {
        let f = fixture(holding("saved"), Arc::new(FakeKeys::default()));
        assert_eq!(f.exchange.selection(), Err(TextError::PermissionMissing));
        assert!(f.keys.events().is_empty());
    }

    struct FakeDirect(Option<String>);

    impl DirectSelection for FakeDirect {
        fn selected_text(&self) -> Option<String> {
            self.0.clone()
        }
    }

    #[test]
    fn the_direct_route_is_preferred_and_touches_nothing() {
        let clipboard = holding("what the user had");
        let keys = FakeKeys::working();
        let exchange = TextExchange::new(
            clipboard.clone(),
            keys.clone(),
            FakeHandoff::working(),
            Some(Arc::new(FakeDirect(Some("selected directly".into())))),
            Arc::new(RecordingWrites::default()),
            Arc::new(FakeLauncher::default()),
        );

        assert_eq!(
            exchange.selection().unwrap().as_deref(),
            Some("selected directly")
        );
        assert!(keys.events().is_empty(), "no copy, no keystroke at all");
        assert_eq!(
            clipboard.now(),
            Some(Content::Text("what the user had".into()))
        );
    }

    #[test]
    fn a_silent_direct_route_falls_back_to_the_clipboard() {
        let clipboard = holding("what the user had");
        let keys = Arc::new(FakeKeys {
            permitted: true,
            pastes_into: Some(clipboard.clone()),
            ..Default::default()
        });
        let exchange = TextExchange::new(
            clipboard,
            keys.clone(),
            FakeHandoff::working(),
            Some(Arc::new(FakeDirect(None))),
            Arc::new(RecordingWrites::default()),
            Arc::new(FakeLauncher::default()),
        );

        assert_eq!(
            exchange.selection().unwrap().as_deref(),
            Some("what was selected")
        );
        assert_eq!(keys.events(), vec!["copy"]);
    }
}

#[cfg(test)]
mod display_tests {
    use super::{TextError, KEYSTROKE_FAILED};

    #[test]
    fn every_message_is_a_sentence() {
        let errors = [
            TextError::PermissionMissing,
            TextError::NoTarget,
            TextError::TargetUnavailable(KEYSTROKE_FAILED.into()),
            TextError::Clipboard,
        ];
        for error in errors {
            let text = error.to_string();
            assert!(text.starts_with(char::is_uppercase), "{text}");
            assert!(text.ends_with('.'), "{text}");
        }
    }
}
