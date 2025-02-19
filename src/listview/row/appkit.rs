//! This module does one specific thing: register a custom `NSView` class that's... brought to the
//! modern era.
//!
//! I kid, I kid.
//!
//! It just enforces that coordinates are judged from the top-left, which is what most people look
//! for in the modern era. It also implements a few helpers for things like setting a background
//! color, and enforcing layer backing by default.

use std::sync::Once;

use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel, BOOL};
use objc::{class, msg_send, sel, sel_impl};
use objc_id::Id;

use crate::dragdrop::DragInfo;
use crate::foundation::{id, nil, NSUInteger, NO, YES};
use crate::listview::row::{ViewDelegate, BACKGROUND_COLOR, LISTVIEW_ROW_DELEGATE_PTR};
use crate::utils::load;

/// Enforces normalcy, or: a needlessly cruel method in terms of the name. You get the idea though.
extern "C" fn enforce_normalcy(_: &Object, _: Sel) -> BOOL {
    return YES;
}

/// Called when a drag/drop operation has entered this view.
extern "C" fn dragging_entered<T: ViewDelegate>(this: &mut Object, _: Sel, info: id) -> NSUInteger {
    let view = load::<T>(this, LISTVIEW_ROW_DELEGATE_PTR);
    view.dragging_entered(DragInfo {
        info: unsafe { Id::from_ptr(info) }
    })
    .into()
}

/// Called when a drag/drop operation has entered this view.
extern "C" fn prepare_for_drag_operation<T: ViewDelegate>(this: &mut Object, _: Sel, info: id) -> BOOL {
    let view = load::<T>(this, LISTVIEW_ROW_DELEGATE_PTR);

    match view.prepare_for_drag_operation(DragInfo {
        info: unsafe { Id::from_ptr(info) }
    }) {
        true => YES,
        false => NO
    }
}

/// Called when a drag/drop operation has entered this view.
extern "C" fn perform_drag_operation<T: ViewDelegate>(this: &mut Object, _: Sel, info: id) -> BOOL {
    let view = load::<T>(this, LISTVIEW_ROW_DELEGATE_PTR);

    match view.perform_drag_operation(DragInfo {
        info: unsafe { Id::from_ptr(info) }
    }) {
        true => YES,
        false => NO
    }
}

/// Called when a drag/drop operation has entered this view.
extern "C" fn conclude_drag_operation<T: ViewDelegate>(this: &mut Object, _: Sel, info: id) {
    let view = load::<T>(this, LISTVIEW_ROW_DELEGATE_PTR);

    view.conclude_drag_operation(DragInfo {
        info: unsafe { Id::from_ptr(info) }
    });
}

/// Called when a drag/drop operation has entered this view.
extern "C" fn dragging_exited<T: ViewDelegate>(this: &mut Object, _: Sel, info: id) {
    let view = load::<T>(this, LISTVIEW_ROW_DELEGATE_PTR);

    view.dragging_exited(DragInfo {
        info: unsafe { Id::from_ptr(info) }
    });
}

/// Called for layer updates.
extern "C" fn update_layer(this: &Object, _: Sel) {
    unsafe {
        let background_color: id = *this.get_ivar(BACKGROUND_COLOR);

        if background_color != nil {
            let layer: id = msg_send![this, layer];
            let cg: id = msg_send![background_color, CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg];
        }
    }
}

/// Normally, you might not want to do a custom dealloc override. However, reusable cells are
/// tricky - since we "forget" them when we give them to the system, we need to make sure to do
/// proper cleanup then the backing (cached) version is deallocated on the Objective-C side. Since
/// we know
extern "C" fn dealloc<T: ViewDelegate>(this: &Object, _: Sel) {
    // Load the Box pointer here, and just let it drop normally.
    unsafe {
        let ptr: usize = *(&*this).get_ivar(LISTVIEW_ROW_DELEGATE_PTR);
        let obj = ptr as *mut T;
        let _x = Box::from_raw(obj);

        let _: () = msg_send![super(this, class!(NSView)), dealloc];
    }
}

/// Injects an `NSView` subclass. This is used for the default views that don't use delegates - we
/// have separate classes here since we don't want to waste cycles on methods that will never be
/// used if there's no delegates.
pub(crate) fn register_listview_row_class() -> *const Class {
    static mut VIEW_CLASS: *const Class = 0 as *const Class;
    static INIT: Once = Once::new();

    INIT.call_once(|| unsafe {
        let superclass = class!(NSView);
        let mut decl = ClassDecl::new("RSTTableViewRow", superclass).unwrap();

        decl.add_method(sel!(isFlipped), enforce_normalcy as extern "C" fn(&Object, _) -> BOOL);

        VIEW_CLASS = decl.register();
    });

    unsafe { VIEW_CLASS }
}

/// Injects an `NSView` subclass, with some callback and pointer ivars for what we
/// need to do.
pub(crate) fn register_listview_row_class_with_delegate<T: ViewDelegate>() -> *const Class {
    static mut VIEW_CLASS: *const Class = 0 as *const Class;
    static INIT: Once = Once::new();

    INIT.call_once(|| unsafe {
        let superclass = class!(NSView);
        let mut decl = ClassDecl::new("RSTableViewRowWithDelegate", superclass).unwrap();

        // A pointer to the "view controller" on the Rust side. It's expected that this doesn't
        // move.
        decl.add_ivar::<usize>(LISTVIEW_ROW_DELEGATE_PTR);
        decl.add_ivar::<id>(BACKGROUND_COLOR);

        decl.add_method(sel!(isFlipped), enforce_normalcy as extern "C" fn(&Object, _) -> BOOL);
        decl.add_method(sel!(updateLayer), update_layer as extern "C" fn(&Object, _));

        // Drag and drop operations (e.g, accepting files)
        decl.add_method(
            sel!(draggingEntered:),
            dragging_entered::<T> as extern "C" fn(&mut Object, _, _) -> NSUInteger
        );
        decl.add_method(
            sel!(prepareForDragOperation:),
            prepare_for_drag_operation::<T> as extern "C" fn(&mut Object, _, _) -> BOOL
        );
        decl.add_method(
            sel!(performDragOperation:),
            perform_drag_operation::<T> as extern "C" fn(&mut Object, _, _) -> BOOL
        );
        decl.add_method(
            sel!(concludeDragOperation:),
            conclude_drag_operation::<T> as extern "C" fn(&mut Object, _, _)
        );
        decl.add_method(
            sel!(draggingExited:),
            dragging_exited::<T> as extern "C" fn(&mut Object, _, _)
        );

        // Cleanup
        decl.add_method(sel!(dealloc), dealloc::<T> as extern "C" fn(&Object, _));

        VIEW_CLASS = decl.register();
    });

    unsafe { VIEW_CLASS }
}
