//! Global mouse, keyboard and scroll listener.
//!
//! Steps are captured when the user interacts with the machine, not when the
//! screen happens to look different. One listener thread runs for the lifetime
//! of the app; recording sessions enable and drain it.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};

use rdev::{Button, Event, EventType, Key};

struct Listener {
    enabled: Arc<AtomicBool>,
    clicked: Arc<AtomicBool>,
    keyed: Arc<AtomicBool>,
    scrolled: Arc<AtomicBool>,
    _handle: JoinHandle<()>,
}

static LISTENER: OnceLock<Listener> = OnceLock::new();

thread_local! {
    static MODIFIERS: RefCell<ModifierState> = RefCell::new(ModifierState::default());
}

#[derive(Default)]
struct ModifierState {
    shift: bool,
    alt: bool,
}

/// Start the global listener once, but only when Accessibility is granted.
/// Returns false when input monitoring is unavailable on this machine.
pub fn ensure_listener() -> bool {
    if !has_input_permission() {
        return false;
    }

    LISTENER.get_or_init(|| {
        let enabled = Arc::new(AtomicBool::new(false));
        let clicked = Arc::new(AtomicBool::new(false));
        let keyed = Arc::new(AtomicBool::new(false));
        let scrolled = Arc::new(AtomicBool::new(false));
        let worker_enabled = Arc::clone(&enabled);
        let worker = Flags {
            clicked: Arc::clone(&clicked),
            keyed: Arc::clone(&keyed),
            scrolled: Arc::clone(&scrolled),
        };

        let handle = thread::Builder::new()
            .name("walkmark-input".into())
            .spawn(move || {
                // rdev defaults to "main thread" mode; Tauri's listener runs on a
                // worker thread and will segfault on keypress without this (macOS).
                #[cfg(target_os = "macos")]
                rdev::set_is_main_thread(false);

                if let Err(error) = rdev::listen(move |event| {
                    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handle_event(&event, &worker_enabled, &worker);
                    }))
                    .is_err()
                    {
                        eprintln!("walkmark input listener dropped an event after a panic");
                    }
                }) {
                    eprintln!("walkmark input listener stopped: {error:?}");
                }
            });

        let handle = match handle {
            Ok(handle) => handle,
            Err(error) => {
                eprintln!("walkmark input listener could not start: {error}");
                return Listener {
                    enabled,
                    clicked,
                    keyed,
                    scrolled,
                    _handle: thread::Builder::new()
                        .spawn(|| {})
                        .expect("spawn noop input listener fallback"),
                };
            }
        };

        Listener {
            enabled,
            clicked,
            keyed,
            scrolled,
            _handle: handle,
        }
    });

    true
}

struct Flags {
    clicked: Arc<AtomicBool>,
    keyed: Arc<AtomicBool>,
    scrolled: Arc<AtomicBool>,
}

fn handle_event(event: &Event, enabled: &AtomicBool, flags: &Flags) {
    if !enabled.load(Ordering::Relaxed) {
        return;
    }
    // Always evaluated: it also tracks which modifier keys are held.
    if !event_triggers_step(event) {
        return;
    }
    match event.event_type {
        EventType::KeyPress(_) => flags.keyed.store(true, Ordering::SeqCst),
        EventType::Wheel { .. } => flags.scrolled.store(true, Ordering::SeqCst),
        EventType::ButtonPress(_) => flags.clicked.store(true, Ordering::SeqCst),
        _ => {}
    }
}

pub fn set_enabled(enabled: bool) {
    if let Some(listener) = LISTENER.get() {
        listener.enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            for flag in [&listener.clicked, &listener.keyed, &listener.scrolled] {
                flag.store(false, Ordering::SeqCst);
            }
        }
    }
}

fn take(pick: impl Fn(&Listener) -> &Arc<AtomicBool>) -> bool {
    LISTENER
        .get()
        .is_some_and(|l| pick(l).swap(false, Ordering::SeqCst))
}

/// A mouse click happened since the last call. Each click gets its own
/// screenshot.
pub fn take_click() -> bool {
    take(|l| &l.clicked)
}

/// A key was pressed since the last call. The screenshot is taken once typing
/// stops, not per key.
pub fn take_key() -> bool {
    take(|l| &l.keyed)
}

/// The wheel moved since the last call.
pub fn take_scroll() -> bool {
    take(|l| &l.scrolled)
}

pub fn has_trigger() -> bool {
    LISTENER.get().is_some_and(|l| {
        [&l.clicked, &l.keyed, &l.scrolled]
            .iter()
            .any(|f| f.load(Ordering::Relaxed))
    })
}

// ---------------------------------------------------------------------------
// macOS Accessibility — required for global input on macOS
// ---------------------------------------------------------------------------

pub fn has_input_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        accessibility_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(target_os = "macos")]
fn accessibility_trusted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(target_os = "macos")]
pub fn request_input_permission() {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    }
    // Passing null prompts the system dialog once.
    unsafe {
        AXIsProcessTrustedWithOptions(std::ptr::null());
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_input_permission() {}

fn event_triggers_step(event: &Event) -> bool {
    MODIFIERS.with(|mods| {
        let mut mods = mods.borrow_mut();
        match event.event_type {
            EventType::ButtonPress(Button::Left | Button::Right | Button::Middle) => true,
            EventType::Wheel { delta_x, delta_y } => delta_x != 0 || delta_y != 0,
            EventType::KeyPress(key) => {
                mods.on_press(key);
                !(mods.is_walkmark_shortcut(key) || is_modifier(key))
            }
            EventType::KeyRelease(key) => {
                mods.on_release(key);
                false
            }
            _ => false,
        }
    })
}

impl ModifierState {
    fn on_press(&mut self, key: Key) {
        match key {
            Key::ShiftLeft | Key::ShiftRight => self.shift = true,
            Key::Alt | Key::AltGr => self.alt = true,
            _ => {}
        }
    }

    fn on_release(&mut self, key: Key) {
        match key {
            Key::ShiftLeft | Key::ShiftRight => self.shift = false,
            Key::Alt | Key::AltGr => self.alt = false,
            _ => {}
        }
    }

    fn is_walkmark_shortcut(&self, key: Key) -> bool {
        self.shift && self.alt && matches!(key, Key::KeyS | Key::KeyM | Key::KeyP)
    }
}

fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::Alt
            | Key::AltGr
            | Key::MetaLeft
            | Key::MetaRight
            | Key::CapsLock
            | Key::NumLock
            | Key::ScrollLock
            | Key::Function
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event_type: EventType) -> Event {
        Event {
            time: std::time::SystemTime::now(),
            name: None,
            event_type,
        }
    }

    #[test]
    fn events_are_sorted_into_click_key_and_scroll() {
        let flags = Flags {
            clicked: Arc::new(AtomicBool::new(false)),
            keyed: Arc::new(AtomicBool::new(false)),
            scrolled: Arc::new(AtomicBool::new(false)),
        };
        let enabled = AtomicBool::new(true);
        let read = |f: &Arc<AtomicBool>| f.swap(false, Ordering::SeqCst);

        handle_event(
            &event(EventType::ButtonPress(Button::Left)),
            &enabled,
            &flags,
        );
        assert!(read(&flags.clicked) && !read(&flags.keyed) && !read(&flags.scrolled));

        handle_event(&event(EventType::KeyPress(Key::KeyA)), &enabled, &flags);
        assert!(read(&flags.keyed) && !read(&flags.clicked) && !read(&flags.scrolled));

        handle_event(
            &event(EventType::Wheel {
                delta_x: 0,
                delta_y: 1,
            }),
            &enabled,
            &flags,
        );
        assert!(read(&flags.scrolled) && !read(&flags.clicked) && !read(&flags.keyed));

        handle_event(
            &event(EventType::ButtonRelease(Button::Left)),
            &enabled,
            &flags,
        );
        assert!(!read(&flags.clicked));
    }

    #[test]
    fn walkmark_shortcuts_are_ignored() {
        MODIFIERS.with(|mods| {
            *mods.borrow_mut() = ModifierState {
                shift: true,
                alt: true,
            };
        });
        assert!(!event_triggers_step(&Event {
            event_type: EventType::KeyPress(Key::KeyM),
            time: std::time::SystemTime::UNIX_EPOCH,
            name: None,
        }));
    }

    #[test]
    fn typing_triggers_a_step() {
        MODIFIERS.with(|mods| mods.borrow_mut().shift = false);
        assert!(event_triggers_step(&Event {
            event_type: EventType::KeyPress(Key::KeyA),
            time: std::time::SystemTime::UNIX_EPOCH,
            name: None,
        }));
    }

    #[test]
    fn clicks_and_scrolls_trigger() {
        assert!(event_triggers_step(&Event {
            event_type: EventType::ButtonPress(Button::Left),
            time: std::time::SystemTime::UNIX_EPOCH,
            name: None,
        }));
        assert!(event_triggers_step(&Event {
            event_type: EventType::Wheel {
                delta_x: 0,
                delta_y: 1,
            },
            time: std::time::SystemTime::UNIX_EPOCH,
            name: None,
        }));
    }
}
