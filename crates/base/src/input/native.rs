use std::{cell::RefCell, collections::HashMap, mem, ptr, sync::Once};

use gpui::Window;
use objc2::{
    ffi, msg_send,
    rc::Retained,
    runtime::{AnyObject, AnyProtocol, Imp, Sel},
    sel,
};
use objc2_foundation::NSString;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::InputContentType;

static INSTALL_TEXT_CONTENT: Once = Once::new();

thread_local! {
    static CONTENT_TYPES: RefCell<HashMap<usize, Retained<NSString>>> = RefCell::new(HashMap::new());
}

/// Synchronize the platform text-content hint used by autofill and password managers.
pub fn set_text_content_type(window: &Window, content_type: Option<InputContentType>) {
    let Some(view) = ns_view(window) else { return };
    INSTALL_TEXT_CONTENT.call_once(|| install_text_content(view));
    if view
        .class()
        .instance_method(sel!(setContentType:))
        .is_none()
    {
        return;
    }

    let value = content_type
        .and_then(InputContentType::ns_text_content_type)
        .map(NSString::from_str);
    let value = value.as_ref().map_or(ptr::null_mut(), |value| {
        Retained::as_ptr(value).cast_mut().cast::<AnyObject>()
    });
    unsafe {
        let _: () = msg_send![view, setContentType: value];
    }
}

fn ns_view(window: &Window) -> Option<&AnyObject> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    Some(unsafe { &*(handle.ns_view.as_ptr() as *const AnyObject) })
}

fn install_text_content(view: &AnyObject) {
    let class = view.class() as *const _ as *mut _;
    unsafe {
        if let Some(protocol) = AnyProtocol::get(c"NSTextContent") {
            ffi::class_addProtocol(class, protocol);
        }
        let getter: Imp = mem::transmute(
            content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel) -> *mut AnyObject,
        );
        let setter: Imp = mem::transmute(
            set_content_type as unsafe extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject),
        );
        ffi::class_addMethod(class, sel!(contentType), getter, c"@@:".as_ptr());
        ffi::class_addMethod(class, sel!(setContentType:), setter, c"v@:@".as_ptr());
    }
}

unsafe extern "C-unwind" fn content_type(this: &AnyObject, _: Sel) -> *mut AnyObject {
    CONTENT_TYPES.with(|values| {
        values
            .borrow()
            .get(&(this as *const _ as usize))
            .map_or(ptr::null_mut(), |value| {
                Retained::as_ptr(value).cast_mut().cast::<AnyObject>()
            })
    })
}

unsafe extern "C-unwind" fn set_content_type(this: &AnyObject, _: Sel, value: *mut AnyObject) {
    CONTENT_TYPES.with(|values| {
        let mut values = values.borrow_mut();
        let key = this as *const _ as usize;
        if value.is_null() {
            values.remove(&key);
        } else if let Some(value) = unsafe { Retained::retain(value.cast::<NSString>()) } {
            values.insert(key, value);
        }
    });
}
