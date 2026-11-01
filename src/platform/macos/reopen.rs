use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AllocAnyThread, DefinedClass};
use objc2_foundation::{NSAppleEventManager, NSObject};

const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
const REOPEN_APPLICATION: u32 = u32::from_be_bytes(*b"rapp");

struct Ivars {
    reopened: Box<dyn Fn()>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and Handler has no Drop.
    #[unsafe(super = NSObject)]
    #[ivars = Ivars]
    struct Handler;

    impl Handler {
        // SAFETY: the signature matches what NSAppleEventManager calls.
        #[unsafe(method(handleReopen:withReplyEvent:))]
        fn handle_reopen(&self, _event: Option<&AnyObject>, _reply: Option<&AnyObject>) {
            (self.ivars().reopened)();
        }
    }
);

pub fn watch_reopen(reopened: Box<dyn Fn()>) {
    let handler: Retained<Handler> = {
        let this = Handler::alloc().set_ivars(Ivars { reopened });
        unsafe { msg_send![super(this), init] }
    };
    let manager = NSAppleEventManager::sharedAppleEventManager();
    unsafe {
        let _: () = msg_send![
            &manager,
            setEventHandler: &*handler,
            andSelector: sel!(handleReopen:withReplyEvent:),
            forEventClass: CORE_EVENT_CLASS,
            andEventID: REOPEN_APPLICATION,
        ];
    }
    // The manager does not retain its handlers.
    std::mem::forget(handler);
}
