// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::Serialize;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Manager, ipc::Channel};

/// Message sent from Rust to the frontend over the channel.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
enum KeystrokeStreamEvent {
    Key(String),
    Error(String),
}

#[derive(Default)]
struct KeystrokeState {
    /// The channel for the current webview. Replaced whenever the frontend
    /// (re)connects, e.g. after a dev-server reload.
    channel: Mutex<Option<Channel<KeystrokeStreamEvent>>>,
    /// The OS listener blocks forever, so only ever start one.
    listener_started: AtomicBool,
}

fn send_event(app: &AppHandle, event: KeystrokeStreamEvent) {
    let state = app.state::<KeystrokeState>();
    if let Ok(channel) = state.channel.lock() {
        if let Some(channel) = channel.as_ref() {
            // Ignore send failures: the webview may have been closed.
            let _ = channel.send(event);
        }
    }
}

fn forward_key(app: &AppHandle, key: String) {
    send_event(app, KeystrokeStreamEvent::Key(key));
}

fn report_error(app: &AppHandle, message: String) {
    eprintln!("[keytastrophe] {message}");
    send_event(app, KeystrokeStreamEvent::Error(message));
    // Allow a later invocation to retry (e.g. after permissions change).
    app.state::<KeystrokeState>()
        .listener_started
        .store(false, Ordering::SeqCst);
}

/// macOS has its own CGEventTap implementation because `rdev` calls Text Input
/// Source APIs from the event-tap thread, which macOS 26+ kills the process for
/// (`dispatch_assert_queue_fail`). `CGEventKeyboardGetUnicodeString` gives us
/// the same layout-aware characters without those APIs.
#[cfg(target_os = "macos")]
mod listener {
    use super::{forward_key, report_error};
    use core_foundation::base::TCFType;
    use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
    use core_graphics::event::{
        CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventType, CallbackResult, EventField,
    };
    use foreign_types::ForeignType;
    use std::ffi::c_void;
    use std::sync::{Arc, Mutex};
    use tauri::AppHandle;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventKeyboardGetUnicodeString(
            event: *mut c_void,
            max_string_length: u64,
            actual_string_length: *mut u64,
            unicode_string: *mut u16,
        );
        fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    }

    pub fn spawn(app: AppHandle) {
        std::thread::Builder::new()
            .name("keystroke-listener".into())
            .spawn(move || listen(app))
            .expect("failed to spawn keystroke listener thread");
    }

    fn listen(app: AppHandle) {
        // Shared so the callback can re-enable the tap if macOS disables it
        // (timeout or secure input). Stored as a raw pointer because the tap
        // cannot reference itself while it is being constructed.
        let tap_port: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
        let tap_port_for_callback = tap_port.clone();
        let app_for_callback = app.clone();

        let tap = unsafe {
            CGEventTap::new_unchecked(
                CGEventTapLocation::HID,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                vec![CGEventType::KeyDown, CGEventType::FlagsChanged],
                move |_proxy, event_type, event| {
                    match event_type {
                        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                            if let Ok(port) = tap_port_for_callback.lock() {
                                if let Some(port) = *port {
                                    CGEventTapEnable(port as *mut c_void, true);
                                }
                            }
                        }
                        _ => {
                            if let Some(key) = key_name(event_type, event) {
                                forward_key(&app_for_callback, key);
                            }
                        }
                    }
                    CallbackResult::Keep
                },
            )
        };

        let tap = match tap {
            Ok(tap) => tap,
            Err(_) => {
                report_error(
                    &app,
                    "Could not create the keyboard event tap. Grant this app \
                     Accessibility and Input Monitoring permission in System \
                     Settings → Privacy & Security, then restart the app."
                        .into(),
                );
                return;
            }
        };

        if let Ok(mut port) = tap_port.lock() {
            *port = Some(tap.mach_port().as_concrete_TypeRef() as usize);
        }

        let source = match tap.mach_port().create_runloop_source(0) {
            Ok(source) => source,
            Err(_) => {
                report_error(&app, "Could not create the event tap run loop.".into());
                return;
            }
        };

        CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });
        tap.enable();
        eprintln!("[keytastrophe] keystroke listener started");
        CFRunLoop::run_current();
    }

    fn key_name(event_type: CGEventType, event: &CGEvent) -> Option<String> {
        let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);

        if matches!(event_type, CGEventType::FlagsChanged) {
            return modifier_name(code, event.get_flags()).map(str::to_string);
        }

        if let Some(name) = special_key_name(code) {
            return Some(name.to_string());
        }

        unicode_text(event).filter(|text| text.chars().any(|c| !c.is_control()))
    }

    fn unicode_text(event: &CGEvent) -> Option<String> {
        let mut buffer = [0u16; 16];
        let mut length: u64 = 0;
        unsafe {
            CGEventKeyboardGetUnicodeString(
                event.as_ptr() as *mut c_void,
                buffer.len() as u64,
                &mut length,
                buffer.as_mut_ptr(),
            );
        }

        if length == 0 {
            return None;
        }

        let end = (length as usize).min(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..end]))
    }

    fn modifier_name(code: i64, flags: CGEventFlags) -> Option<&'static str> {
        let (name, flag) = match code {
            56 | 60 => ("Shift", CGEventFlags::CGEventFlagShift),
            59 | 62 => ("Control", CGEventFlags::CGEventFlagControl),
            58 | 61 => ("Option", CGEventFlags::CGEventFlagAlternate),
            55 | 54 => ("Command", CGEventFlags::CGEventFlagCommand),
            57 => ("Caps Lock", CGEventFlags::CGEventFlagAlphaShift),
            63 => ("Fn", CGEventFlags::CGEventFlagSecondaryFn),
            _ => return None,
        };

        flags.contains(flag).then_some(name)
    }

    fn special_key_name(code: i64) -> Option<&'static str> {
        Some(match code {
            36 | 76 => "Return",
            48 => "Tab",
            49 => "Space",
            51 => "Delete",
            53 => "Esc",
            117 => "Forward Delete",
            115 => "Home",
            119 => "End",
            116 => "Page Up",
            121 => "Page Down",
            123 => "←",
            124 => "→",
            125 => "↓",
            126 => "↑",
            122 => "F1",
            120 => "F2",
            99 => "F3",
            118 => "F4",
            96 => "F5",
            97 => "F6",
            98 => "F7",
            100 => "F8",
            101 => "F9",
            109 => "F10",
            103 => "F11",
            111 => "F12",
            105 => "F13",
            107 => "F14",
            113 => "F15",
            _ => return None,
        })
    }
}

/// Windows/Linux still use `rdev`, which is fine there. On macOS `rdev::listen`
/// crashes as described above, hence the separate implementation.
#[cfg(not(target_os = "macos"))]
mod listener {
    use super::{forward_key, report_error};
    use rdev::{EventType, listen};
    use tauri::AppHandle;

    pub fn spawn(app: AppHandle) {
        std::thread::Builder::new()
            .name("keystroke-listener".into())
            .spawn(move || {
                let app_for_events = app.clone();
                let result = listen(move |event| {
                    if !matches!(event.event_type, EventType::KeyPress(_)) {
                        return;
                    }

                    let Some(name) = event.name else {
                        return;
                    };
                    if !name.is_empty() {
                        forward_key(&app_for_events, name);
                    }
                });

                if let Err(error) = result {
                    report_error(&app, format!("Keystroke listener failed: {error:?}"));
                }
            })
            .expect("failed to spawn keystroke listener thread");
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn start_keystroke_stream(
    app: AppHandle,
    channel: Channel<KeystrokeStreamEvent>,
) -> Result<(), String> {
    let state = app.state::<KeystrokeState>();
    *state.channel.lock().map_err(|error| error.to_string())? = Some(channel);

    // Already listening: just keep streaming into the new channel.
    if state.listener_started.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    listener::spawn(app);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(KeystrokeState::default())
        .invoke_handler(tauri::generate_handler![greet, start_keystroke_stream])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
