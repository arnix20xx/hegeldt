//! macOS menu bar (NSStatusItem) support.
//!
//! GPUI has no status-item API, so this talks to AppKit directly through the `objc` runtime
//! (the same `objc`/`cocoa` versions `gpui_macos` uses). All unsafe code lives here. Every
//! function must be called on the main thread, which is where GPUI runs its callbacks.
//!
//! IMPORTANT: AppKit callbacks into GPUI (window key/activation changes) borrow the GPUI `App`.
//! Do not call these functions from inside a GPUI `update`/event handler; run them from a
//! spawned foreground task instead (see `RemoteApp::toggle_menu_bar`), or GPUI panics with
//! "RefCell already borrowed".
//!
//! * left click on the status item toggles the remote window (and brings it to the front),
//! * right click (or ctrl-click) opens a small menu with "Show / Hide" and "Quit".

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether this platform has a menu bar mode. On other platforms every function in this module
/// is a no-op and the setting is hidden in the UI (the config field is kept for compatibility).
pub const SUPPORTED: bool = cfg!(target_os = "macos");

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Whether the menu bar mode is currently active.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Turns the menu bar mode on or off: adds/removes the status item and switches the Dock
/// icon off/on (activation policy Accessory/Regular).
pub fn set_enabled(on: bool) {
    if on == enabled() {
        return;
    }
    ENABLED.store(on, Ordering::Relaxed);
    imp::set_enabled(on);
}

/// Hides the window (the app keeps running in the menu bar).
pub fn hide_window() {
    imp::hide_window();
}

#[cfg(target_os = "macos")]
mod imp {
    use std::cell::RefCell;

    use cocoa::appkit::NSApp;
    use cocoa::base::{BOOL, NO, YES, id, nil};
    use cocoa::foundation::NSString;
    use objc::declare::ClassDecl;
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};

    const POLICY_REGULAR: i64 = 0;
    const POLICY_ACCESSORY: i64 = 1;
    /// NSEventMaskLeftMouseUp | NSEventMaskRightMouseUp
    const ACTION_MASK: u64 = (1 << 2) | (1 << 4);
    const EVENT_RIGHT_MOUSE_UP: u64 = 4;
    const MODIFIER_CONTROL: u64 = 1 << 18;
    const VARIABLE_LENGTH: f64 = -1.0;

    thread_local! {
        /// The installed status item (retained), if any.
        static ITEM: RefCell<Option<id>> = const { RefCell::new(None) };
    }

    unsafe fn ns_string(s: &str) -> id {
        unsafe { NSString::alloc(nil).init_str(s) }
    }

    /// The windows that can take focus, i.e. not the status bar's own window.
    unsafe fn app_windows() -> Vec<id> {
        unsafe {
            let windows: id = msg_send![NSApp(), windows];
            let count: usize = msg_send![windows, count];
            (0..count)
                .map(|i| -> id { msg_send![windows, objectAtIndex: i] })
                .filter(|w| {
                    let ok: BOOL = msg_send![*w, canBecomeKeyWindow];
                    ok != NO
                })
                .collect()
        }
    }

    fn show_window() {
        unsafe {
            let _: () = msg_send![NSApp(), activateIgnoringOtherApps: YES];
            for w in app_windows() {
                let _: () = msg_send![w, makeKeyAndOrderFront: nil];
            }
        }
    }

    pub fn hide_window() {
        unsafe {
            for w in app_windows() {
                let _: () = msg_send![w, orderOut: nil];
            }
        }
    }

    fn toggle_window() {
        unsafe {
            let active: BOOL = msg_send![NSApp(), isActive];
            let visible = app_windows().iter().any(|w| {
                let v: BOOL = msg_send![*w, isVisible];
                v != NO
            });
            if visible && active != NO {
                hide_window();
            } else {
                show_window();
            }
        }
    }

    extern "C" fn on_click(this: &Object, _: Sel, _: id) {
        unsafe {
            let event: id = msg_send![NSApp(), currentEvent];
            let (ty, flags): (u64, u64) = if event == nil {
                (0, 0)
            } else {
                (msg_send![event, type], msg_send![event, modifierFlags])
            };
            if ty == EVENT_RIGHT_MOUSE_UP || flags & MODIFIER_CONTROL != 0 {
                // Attach the menu only for this click so left click keeps toggling.
                let item: id = *this.get_ivar("item");
                let menu: id = *this.get_ivar("menu");
                let button: id = msg_send![item, button];
                let _: () = msg_send![item, setMenu: menu];
                let _: () = msg_send![button, performClick: nil];
                let _: () = msg_send![item, setMenu: nil];
            } else {
                toggle_window();
            }
        }
    }

    extern "C" fn on_toggle(_: &Object, _: Sel, _: id) {
        toggle_window();
    }

    extern "C" fn on_quit(_: &Object, _: Sel, _: id) {
        unsafe {
            let _: () = msg_send![NSApp(), terminate: nil];
        }
    }

    fn target_class() -> &'static Class {
        if let Some(c) = Class::get("HegeldtStatusTarget") {
            return c;
        }
        let mut decl =
            ClassDecl::new("HegeldtStatusTarget", class!(NSObject)).expect("class name is unique");
        decl.add_ivar::<id>("item");
        decl.add_ivar::<id>("menu");
        unsafe {
            decl.add_method(
                sel!(statusClick:),
                on_click as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(
                sel!(toggleWindow:),
                on_toggle as extern "C" fn(&Object, Sel, id),
            );
            decl.add_method(sel!(quitApp:), on_quit as extern "C" fn(&Object, Sel, id));
        }
        decl.register()
    }

    unsafe fn menu_item(title: &str, action: Sel, key: &str, target: id) -> id {
        unsafe {
            let mi: id = msg_send![class!(NSMenuItem), alloc];
            let mi: id = msg_send![mi, initWithTitle: ns_string(title) action: action keyEquivalent: ns_string(key)];
            let _: () = msg_send![mi, setTarget: target];
            mi
        }
    }

    fn install() {
        ITEM.with(|slot| {
            if slot.borrow().is_some() {
                return;
            }
            unsafe {
                let bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
                let item: id = msg_send![bar, statusItemWithLength: VARIABLE_LENGTH];
                let item: id = msg_send![item, retain];
                let button: id = msg_send![item, button];

                // SF Symbol as a template image (adapts to light/dark menu bar); fall back to text.
                let has_symbols: BOOL = msg_send![class!(NSImage), respondsToSelector: sel!(imageWithSystemSymbolName:accessibilityDescription:)];
                let image: id = if has_symbols != NO {
                    msg_send![class!(NSImage), imageWithSystemSymbolName: ns_string("slider.horizontal.3") accessibilityDescription: ns_string("Hegel Remote")]
                } else {
                    nil
                };
                if image == nil {
                    let _: () = msg_send![button, setTitle: ns_string("Hegel")];
                } else {
                    let _: () = msg_send![image, setTemplate: YES];
                    let _: () = msg_send![button, setImage: image];
                }

                let target: id = msg_send![target_class(), new];
                let menu: id = msg_send![class!(NSMenu), new];
                let _: () = msg_send![menu, addItem: menu_item("Show / Hide Hegel Remote", sel!(toggleWindow:), "", target)];
                let _: () = msg_send![menu, addItem: menu_item("Quit Hegel Remote", sel!(quitApp:), "q", target)];
                (*target).set_ivar("item", item);
                (*target).set_ivar("menu", menu);

                let _: () = msg_send![button, setTarget: target];
                let _: () = msg_send![button, setAction: sel!(statusClick:)];
                let _: u64 = msg_send![button, sendActionOn: ACTION_MASK];
                *slot.borrow_mut() = Some(item);
            }
        });
    }

    fn uninstall() {
        ITEM.with(|slot| {
            if let Some(item) = slot.borrow_mut().take() {
                unsafe {
                    let bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
                    let _: () = msg_send![bar, removeStatusItem: item];
                    let _: () = msg_send![item, release];
                }
            }
        });
    }

    pub fn set_enabled(on: bool) {
        unsafe {
            if on {
                install();
                let _: BOOL = msg_send![NSApp(), setActivationPolicy: POLICY_ACCESSORY];
            } else {
                uninstall();
                let _: BOOL = msg_send![NSApp(), setActivationPolicy: POLICY_REGULAR];
            }
            // An accessory app does not come to the front on its own; and when leaving the
            // mode the window should stay visible.
            show_window();
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn hide_window() {}
    pub fn set_enabled(_on: bool) {}
}
