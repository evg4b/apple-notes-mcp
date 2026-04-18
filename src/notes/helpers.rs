use objc2::__framework_prelude::{AnyObject, Retained};
use objc2::msg_send;
use objc2::runtime::Sel;
use objc2_foundation::{NSArray, NSString};

/// Read a scripting property as a plain `String` via KVC + `description`.
/// Works for NSString (returns the string itself) and NSDate (returns its description).
pub(super) unsafe fn kvc_string(obj: &AnyObject, key: &str) -> String {
    match unsafe { kvc_get(obj, key) } {
        None => String::new(),
        Some(val) => {
            let desc: Retained<NSString> = unsafe { msg_send![&*val, description] };
            desc.to_string()
        }
    }
}

/// Read a boolean scripting property via KVC.
/// ScriptingBridge returns boolean values as NSNumber; `charValue` is a real
/// NSNumber method, so the debug assertion passes.
pub(super) unsafe fn kvc_bool(obj: &AnyObject, key: &str) -> bool {
    match unsafe { kvc_get(obj, key) } {
        None => false,
        Some(val) => {
            let n: i8 = unsafe { msg_send![&*val, charValue] };
            n != 0
        }
    }
}

/// Get a scripting-dictionary property via KVC (`valueForKey:`).
///
/// `SBObject` implements `valueForKey:` to send Apple Events, so this works for
/// any property in the Notes scripting dictionary without the selector needing to
/// appear in the static method table.
pub(super) unsafe fn kvc_get(obj: &AnyObject, key: &str) -> Option<Retained<AnyObject>> {
    let key_ns = NSString::from_str(key);
    unsafe { msg_send![obj, valueForKey: &*key_ns] }
}

/// Set a string scripting property via KVC (`setValue:forKey:`).
pub(super) unsafe fn kvc_set(obj: &AnyObject, key: &str, value: &str) {
    let key_ns = NSString::from_str(key);
    let val_ns = NSString::from_str(value);
    let _: () = unsafe { msg_send![obj, setValue: &*val_ns, forKey: &*key_ns] };
}

/// Return the number of elements in a ScriptingBridge element array.
pub(super) unsafe fn sb_count(arr: &AnyObject) -> usize {
    unsafe { msg_send![arr, count] }
}

/// Return the element at `index` in a ScriptingBridge element array.
pub(super) unsafe fn sb_at(arr: &AnyObject, index: usize) -> Retained<AnyObject> {
    unsafe { msg_send![arr, objectAtIndex: index] }
}

/// Batch-fetch a string property from every element in an SBElementArray.
///
/// Calls `valueForKey:` on the *collection* (not individual elements), which
/// ScriptingBridge translates into a single "get all elements' <key>" Apple
/// Event. The result is a plain `NSArray` — iteration is then local with no
/// further Apple Events. Cost: O(1) Apple Events instead of O(2N).
pub(super) unsafe fn kvc_string_vec(collection: &AnyObject, key: &str) -> Vec<String> {
    let Some(raw) = (unsafe { kvc_get(collection, key) }) else {
        return Vec::new();
    };
    // valueForKey: on an SBElementArray returns a plain NSArray.
    // Downcast so we can use the safe NSArray::iter() from objc2-foundation
    // instead of manual index arithmetic with msg_send!.
    let Some(arr) = raw.downcast_ref::<NSArray<AnyObject>>() else {
        return Vec::new();
    };
    arr.iter()
        .map(|elem| {
            // iter() yields Retained<AnyObject>; &*elem coerces to &AnyObject for msg_send!
            let desc: Retained<NSString> = unsafe { msg_send![&*elem, description] };
            desc.to_string()
        })
        .collect()
}

/// Batch-fetch a boolean property from every element in an SBElementArray.
///
/// Same single-Apple-Event strategy as `kvc_string_vec`.
pub(super) unsafe fn kvc_bool_vec(collection: &AnyObject, key: &str) -> Vec<bool> {
    let Some(raw) = (unsafe { kvc_get(collection, key) }) else {
        return Vec::new();
    };
    let Some(arr) = raw.downcast_ref::<NSArray<AnyObject>>() else {
        return Vec::new();
    };
    arr.iter()
        .map(|elem| {
            let n: i8 = unsafe { msg_send![&*elem, charValue] };
            n != 0
        })
        .collect()
}

/// Retrieve a ScriptingBridge element collection via `performSelector:`.
///
/// ScriptingBridge routes collection selectors (`notes`, `folders`, `accounts`,
/// `attachments`) through `doesNotUnderstand:`, so they are absent from the static
/// method table. objc2's debug assertions call `responds_to_selector:` before every
/// `msg_send!`, which returns NO for those selectors. Routing through
/// `performSelector:` (a real NSObject method) bypasses that check and lets the
/// ObjC runtime dispatch dynamically.
pub(super) unsafe fn sb_collection(obj: &AnyObject, sel: Sel) -> Retained<AnyObject> {
    unsafe { msg_send![obj, performSelector: sel] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2::class;
    use objc2_foundation::NSNumber;

    unsafe fn new_dict() -> Retained<AnyObject> {
        let alloc: *mut AnyObject = msg_send![class!(NSMutableDictionary), alloc];
        let init: *mut AnyObject = msg_send![alloc, init];
        unsafe { Retained::from_raw(init).unwrap() }
    }

    unsafe fn dict_set_str(obj: &AnyObject, key: &str, val: &str) {
        let k = NSString::from_str(key);
        let v = NSString::from_str(val);
        let _: () = msg_send![obj, setValue: &*v, forKey: &*k];
    }

    unsafe fn dict_set_bool(obj: &AnyObject, key: &str, val: bool) {
        let k = NSString::from_str(key);
        let n: Retained<NSNumber> = msg_send![class!(NSNumber), numberWithBool: val];
        let _: () = msg_send![obj, setValue: &*n, forKey: &*k];
    }

    unsafe fn make_nsarray(items: &[&str]) -> Retained<AnyObject> {
        if items.is_empty() {
            return msg_send![class!(NSArray), array];
        }
        let strings: Vec<Retained<NSString>> =
            items.iter().map(|&s| NSString::from_str(s)).collect();
        let ptrs: Vec<*const AnyObject> = strings
            .iter()
            .map(|s| s.as_ref() as *const NSString as *const AnyObject)
            .collect();
        msg_send![class!(NSArray), arrayWithObjects: ptrs.as_ptr(), count: ptrs.len()]
    }

    #[test]
    fn kvc_get_returns_none_for_missing_key() {
        unsafe {
            let dict = new_dict();
            assert!(kvc_get(&dict, "missing").is_none());
        }
    }

    #[test]
    fn kvc_string_returns_empty_for_missing_key() {
        unsafe {
            let dict = new_dict();
            assert_eq!(kvc_string(&dict, "missing"), "");
        }
    }

    #[test]
    fn kvc_string_returns_stored_value() {
        unsafe {
            let dict = new_dict();
            dict_set_str(&dict, "name", "Alice");
            assert_eq!(kvc_string(&dict, "name"), "Alice");
        }
    }

    #[test]
    fn kvc_bool_returns_false_for_missing_key() {
        unsafe {
            let dict = new_dict();
            assert!(!kvc_bool(&dict, "flag"));
        }
    }

    #[test]
    fn kvc_bool_returns_true() {
        unsafe {
            let dict = new_dict();
            dict_set_bool(&dict, "active", true);
            assert!(kvc_bool(&dict, "active"));
        }
    }

    #[test]
    fn kvc_bool_returns_false() {
        unsafe {
            let dict = new_dict();
            dict_set_bool(&dict, "active", false);
            assert!(!kvc_bool(&dict, "active"));
        }
    }

    #[test]
    fn kvc_set_stores_value() {
        unsafe {
            let dict = new_dict();
            kvc_set(&dict, "body", "content");
            assert_eq!(kvc_string(&dict, "body"), "content");
        }
    }

    #[test]
    fn kvc_set_overwrites_existing_value() {
        unsafe {
            let dict = new_dict();
            kvc_set(&dict, "title", "Old");
            kvc_set(&dict, "title", "New");
            assert_eq!(kvc_string(&dict, "title"), "New");
        }
    }

    #[test]
    fn sb_count_empty_array() {
        unsafe {
            let arr = make_nsarray(&[]);
            assert_eq!(sb_count(&arr), 0);
        }
    }

    #[test]
    fn sb_count_returns_length() {
        unsafe {
            let arr = make_nsarray(&["a", "b", "c"]);
            assert_eq!(sb_count(&arr), 3);
        }
    }

    #[test]
    fn sb_at_returns_element_at_index() {
        unsafe {
            let arr = make_nsarray(&["first", "second"]);
            let elem = sb_at(&arr, 0);
            let desc: Retained<NSString> = msg_send![&*elem, description];
            assert_eq!(desc.to_string(), "first");
        }
    }
}
