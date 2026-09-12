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

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum TextError {
    #[error("Dango needs the Accessibility permission to do that")]
    PermissionMissing,
    #[error("there is no application to put that into")]
    NoTarget,
    #[error("{0}")]
    TargetUnavailable(String),
    #[error("the clipboard could not be used")]
    Clipboard,
}

/// The keystrokes, per platform: `Cmd` on macOS, `Ctrl` on Windows, and the
/// layout mapping underneath that neither of them should own.
pub trait Keys: Send + Sync {
    fn copy(&self) -> Result<(), TextError>;
    fn paste(&self) -> Result<(), TextError>;
    fn caret_left(&self, times: usize) -> Result<(), TextError>;
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
    fn selection(&self) -> Result<Option<String>, TextError>;
    fn clipboard_text(&self) -> Option<String>;
}

impl TextTarget for TextExchange {
    fn insert(&self, text: &str, caret: Option<usize>) -> Result<(), TextError> {
        TextExchange::insert(self, text, caret)
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
}

impl TextExchange {
    pub fn new(
        clipboard: Arc<dyn ClipboardSource>,
        keys: Arc<dyn Keys>,
        handoff: Arc<dyn Handoff>,
        direct: Option<Arc<dyn DirectSelection>>,
        own_writes: Arc<dyn OwnWrites>,
    ) -> Self {
        Self {
            clipboard,
            keys,
            handoff,
            direct,
            own_writes,
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

        self.handoff.yield_to_previous()?;

        let sentinel = format!("dango-selection-{}", uuid::Uuid::new_v4());
        self.own_writes.expect(&Content::Text(sentinel.clone()));
        self.clipboard.set_text(&sentinel);

        let after_copy = self.keys.copy().map(|()| {
            self.handoff.settle_after_paste();
            self.clipboard.text()
        })?;

        // One read, not two: anything that appears after this point came from
        // the user, and re-reading would hand them their own content back as
        // the thing Dango borrowed.
        let borrowed = after_copy.clone().unwrap_or_else(|| sentinel.clone());
        self.restore(saved, &borrowed);

        Ok(after_copy.filter(|text| *text != sentinel && !text.is_empty()))
    }

    /// Puts text into the application the user was in. The caret, when given,
    /// is a character offset into `text` where it should end up.
    pub fn insert(&self, text: &str, caret: Option<usize>) -> Result<(), TextError> {
        if !self.keys.permitted() {
            return Err(TextError::PermissionMissing);
        }

        let saved = self.read_clipboard();
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
        match saved {
            Content::Text(text) => self.clipboard.set_text(&text),
            Content::Image(png) => self.clipboard.set_image(&png),
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
    }

    fn fixture(clipboard: Arc<FakeClipboard>, keys: Arc<FakeKeys>) -> Fixture {
        let handoff = FakeHandoff::working();
        let writes = Arc::new(RecordingWrites::default());
        Fixture {
            exchange: TextExchange::new(
                clipboard.clone(),
                keys.clone(),
                handoff.clone(),
                None,
                writes.clone(),
            ),
            clipboard,
            keys,
            handoff,
            writes,
        }
    }

    fn holding(text: &str) -> Arc<FakeClipboard> {
        FakeClipboard::holding(Content::Text(text.into()))
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
        );

        assert_eq!(
            exchange.selection().unwrap().as_deref(),
            Some("what was selected")
        );
        assert_eq!(keys.events(), vec!["copy"]);
    }
}
