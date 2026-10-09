use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_core_services::AppleEvent;
use objc2_foundation::{NSError, NSObject, NSString, ns_string};
use objc2_scripting_bridge::SBApplicationDelegate;
use std::cell::RefCell;
use std::ptr::{self, NonNull};

#[derive(Default)]
pub(super) struct EventErrors {
    first: RefCell<Option<String>>,
}

define_class!(
    /// Without a delegate, `SBApplication` raises an Objective-C exception for
    /// every failed Apple Event, and an exception cannot unwind through Rust
    /// frames: the process aborts. This one records the failure instead and
    /// lets the call return nil, so the operation can report it as an error.
    #[unsafe(super(NSObject))]
    #[name = "AppleNotesMCPEventErrorDelegate"]
    #[ivars = EventErrors]
    pub(super) struct EventErrorDelegate;

    unsafe impl NSObjectProtocol for EventErrorDelegate {}

    unsafe impl SBApplicationDelegate for EventErrorDelegate {
        #[unsafe(method(eventDidFail:withError:))]
        fn event_did_fail(&self, _event: NonNull<AppleEvent>, error: &NSError) -> *mut AnyObject {
            let mut first = self.ivars().first.borrow_mut();
            // Later failures in the same operation are usually knock-on
            // effects of the first one.
            if first.is_none() {
                *first = Some(describe(error));
            }
            ptr::null_mut()
        }
    }
);

impl EventErrorDelegate {
    pub(super) fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(EventErrors::default());
        unsafe { msg_send![super(this), init] }
    }

    /// The first Apple Event failure since the last call, if any.
    pub(super) fn take_error(&self) -> Option<String> {
        self.ivars().first.take()
    }
}

fn describe(error: &NSError) -> String {
    let brief = error
        .userInfo()
        .objectForKey(ns_string!("ErrorBriefMessage"))
        .and_then(|value| value.downcast::<NSString>().ok());
    let message = match brief {
        Some(brief) => brief.to_string(),
        None => error.localizedDescription().to_string(),
    };
    describe_code(error.code(), &message)
}

/// The OSStatus codes a Notes call realistically hits, with what to do about
/// them; the raw messages for these are generic.
fn describe_code(code: isize, message: &str) -> String {
    let hint = match code {
        -1743 => Some(
            "this binary is not allowed to control Notes.app; allow it in System Settings → \
             Privacy & Security → Automation",
        ),
        -1712 => Some("Notes.app did not respond in time"),
        -1728 | -1719 => Some("the note or folder is gone; it may have changed in Notes meanwhile"),
        -600 | -609 => Some("Notes.app is not running and could not be reached"),
        _ => None,
    };
    match hint {
        Some(hint) => format!("Notes.app error {code}: {hint} ({message})"),
        None => format!("Notes.app error {code}: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_foundation::{NSDictionary, NSErrorDomain};

    fn ns_error(code: isize, brief: Option<&str>) -> Retained<NSError> {
        let domain: &NSErrorDomain = ns_string!("NSOSStatusErrorDomain");
        let user_info = match brief {
            Some(brief) => {
                let value = NSString::from_str(brief);
                NSDictionary::<NSString, AnyObject>::from_slices(
                    &[ns_string!("ErrorBriefMessage")],
                    &[value.as_ref()],
                )
            }
            None => NSDictionary::new(),
        };
        unsafe { NSError::errorWithDomain_code_userInfo(domain, code, Some(&user_info)) }
    }

    fn fail(delegate: &EventErrorDelegate, error: &NSError) {
        let event = NonNull::<AppleEvent>::dangling();
        let _: *mut AnyObject =
            unsafe { msg_send![delegate, eventDidFail: event, withError: error] };
    }

    #[test]
    fn no_failure_means_no_error() {
        assert_eq!(EventErrorDelegate::new().take_error(), None);
    }

    #[test]
    fn records_a_failed_event() {
        let delegate = EventErrorDelegate::new();
        fail(&delegate, &ns_error(-1728, Some("Can’t get note 1.")));
        let error = delegate.take_error().expect("failure was not recorded");
        assert!(error.contains("-1728"), "{error}");
        assert!(error.contains("Can’t get note 1."), "{error}");
    }

    #[test]
    fn take_clears_the_error() {
        let delegate = EventErrorDelegate::new();
        fail(&delegate, &ns_error(-1712, None));
        assert!(delegate.take_error().is_some());
        assert_eq!(delegate.take_error(), None);
    }

    #[test]
    fn keeps_the_first_of_several_failures() {
        let delegate = EventErrorDelegate::new();
        fail(&delegate, &ns_error(-1743, None));
        fail(&delegate, &ns_error(-1728, None));
        let error = delegate.take_error().unwrap();
        assert!(error.contains("-1743"), "{error}");
    }

    #[test]
    fn falls_back_to_the_localized_description() {
        let delegate = EventErrorDelegate::new();
        fail(&delegate, &ns_error(-50, None));
        let error = delegate.take_error().unwrap();
        assert!(error.starts_with("Notes.app error -50: "), "{error}");
        assert!(error.len() > "Notes.app error -50: ".len(), "{error}");
    }

    #[test]
    fn permission_errors_say_how_to_fix_them() {
        let message = describe_code(-1743, "Not authorized to send Apple events to Notes.");
        assert!(message.contains("Automation"), "{message}");
        assert!(message.contains("Not authorized"), "{message}");
    }
}
