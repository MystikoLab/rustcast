use std::{
    ffi::c_void,
    ptr::{NonNull, null_mut},
    sync::{Arc, Mutex},
};

use objc2_app_kit::NSEventModifierFlags;
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};

use crate::{
    app::{Message, tile::ExtSender},
    platform::macos::accessibility::ensure_accessibility_permission,
};
use iced::keyboard::{
    Modifiers,
    key::{Code, Physical},
};

#[derive(Clone, Debug)]
pub struct EventTapHandle {
    tap_port: CFRetained<CFMachPort>,
    loop_source: CFRetained<CFRunLoopSource>,
    callback_data: *mut c_void,
}

impl Drop for EventTapHandle {
    fn drop(&mut self) {
        CGEvent::tap_enable(&self.tap_port, false);

        let run_loop = CFRunLoop::main().expect("Failed to get main CFRunLoop");
        run_loop.remove_source(Some(&self.loop_source), unsafe { kCFRunLoopCommonModes });

        // Free the callback data
        if !self.callback_data.is_null() {
            unsafe {
                drop(Box::from_raw(self.callback_data as *mut CallbackData));
            }
        }
    }
}

extern "C-unwind" fn keyboard_event_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    mut event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    if user_info.is_null() {
        log::error!("Null user_info in keyboard_event_callback");
        return unsafe { event.as_mut() };
    }

    let data = unsafe { &*(user_info as *const CallbackData) };

    let key_code: u16 = unsafe {
        CGEvent::integer_value_field(Some(event.as_ref()), CGEventField::KeyboardEventKeycode)
    } as u16;

    let flags: CGEventFlags = unsafe { CGEvent::flags(Some(event.as_ref())) };

    let mut mods = NSEventModifierFlags::empty();

    if flags.contains(CGEventFlags::MaskCommand) {
        mods |= NSEventModifierFlags::Command;
    }
    if flags.contains(CGEventFlags::MaskAlternate) {
        mods |= NSEventModifierFlags::Option;
    }
    if flags.contains(CGEventFlags::MaskControl) {
        mods |= NSEventModifierFlags::Control;
    }
    if flags.contains(CGEventFlags::MaskShift) {
        mods |= NSEventModifierFlags::Shift;
    }
    if flags.contains(CGEventFlags::MaskAlphaShift) {
        mods |= NSEventModifierFlags::CapsLock;
    }
    if flags.contains(CGEventFlags::MaskSecondaryFn) {
        mods |= NSEventModifierFlags::Function;
    }

    let shortcut = match event_type {
        CGEventType::KeyDown => Shortcut {
            key_code: Some(key_code),
            mods: if mods.0 != 0 { Some(mods.0) } else { None },
        },
        CGEventType::FlagsChanged => {
            let is_press = match key_code {
                56 | 60 => flags.contains(CGEventFlags::MaskShift), // LSHIFT | RSHIFT
                59 | 62 => flags.contains(CGEventFlags::MaskControl), // LCTRL  | RCTRL
                58 | 61 => flags.contains(CGEventFlags::MaskAlternate), // LOPT   | ROPT
                55 | 54 => flags.contains(CGEventFlags::MaskCommand), // LCMD   | RCMD
                63 => flags.contains(CGEventFlags::MaskSecondaryFn), // FN
                57 => flags.contains(CGEventFlags::MaskAlphaShift), // CAPSLOCK
                _ => false,
            };

            if !is_press {
                return unsafe { event.as_mut() };
            }

            let self_flag = match key_code {
                56 | 60 => NSEventModifierFlags::Shift,   // LSHIFT | RSHIFT
                59 | 62 => NSEventModifierFlags::Control, // LCTRL  | RCTRL
                58 | 61 => NSEventModifierFlags::Option,  // LOPT   | ROPT
                55 | 54 => NSEventModifierFlags::Command, // LCMD   | RCMD
                63 => NSEventModifierFlags::Function,     // FN
                57 => NSEventModifierFlags::CapsLock,     // CAPSLOCK
                _ => NSEventModifierFlags::empty(),
            };

            mods.remove(self_flag);

            Shortcut {
                key_code: Some(key_code),
                mods: if mods.is_empty() { None } else { Some(mods.0) },
            }
        }
        _ => return unsafe { event.as_mut() },
    };

    if !data.targets.contains(&shortcut) {
        return unsafe { event.as_mut() };
    }

    if let Ok(mut sender) = data.sender.lock() {
        sender.0.try_send(Message::KeyPressed(shortcut)).unwrap();
    }

    null_mut()
}

pub struct CallbackData {
    sender: Arc<Mutex<ExtSender>>,
    targets: Vec<Shortcut>,
}

pub fn global_handler(sender: ExtSender, targets: Vec<Shortcut>) -> Result<EventTapHandle, String> {
    ensure_accessibility_permission(); // make it return Result

    let callback_data = Box::new(CallbackData {
        sender: Arc::new(Mutex::new(sender)),
        targets,
    });
    let user_info = Box::into_raw(callback_data) as *mut c_void;

    let mask =
        (1u64 << CGEventType::KeyDown.0 as u64) | (1u64 << CGEventType::FlagsChanged.0 as u64);

    let tap_port = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            mask,
            Some(keyboard_event_callback),
            user_info,
        )
    }
    .unwrap();

    let loop_source = CFMachPort::new_run_loop_source(None, Some(&tap_port), 0)
        .ok_or_else(|| "Failed to create run loop source".to_string())?;

    let run_loop = CFRunLoop::main().ok_or_else(|| "Failed to get main run loop".to_string())?;
    run_loop.add_source(Some(&loop_source), unsafe { kCFRunLoopCommonModes });

    CGEvent::tap_enable(&tap_port, true);

    Ok(EventTapHandle {
        tap_port,
        loop_source,
        callback_data: user_info,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key_code: Option<u16>,
    pub mods: Option<usize>,
}

impl Shortcut {
    pub fn new(key_code: Option<u16>, mods: Option<usize>) -> Self {
        Self { key_code, mods }
    }

    pub fn parse(s: &str) -> Result<Shortcut, String> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        let mut mods: usize = 0;
        let mut key_code: Option<u16> = None;
        let mut has_mods = false;

        for part in &parts {
            match part.to_lowercase().as_str() {
                "cmd" | "command" | "super" => {
                    mods |= NSEventModifierFlags::Command.0;
                    has_mods = true;
                }
                "opt" | "option" | "alt" => {
                    mods |= NSEventModifierFlags::Option.0;
                    has_mods = true;
                }
                "capslock" | "caps" | "caps lock" => mods |= NSEventModifierFlags::CapsLock.0,
                "ctrl" | "control" => {
                    mods |= NSEventModifierFlags::Control.0;
                    has_mods = true;
                }
                "shift" => {
                    mods |= NSEventModifierFlags::Shift.0;
                    has_mods = true;
                }
                "fn" | "function" => {
                    mods |= NSEventModifierFlags::Function.0;
                    has_mods = true;
                }
                key => {
                    if key_code.is_some() {
                        return Err(format!("Multiple keys specified: '{}'", s));
                    }
                    key_code = Some(str_to_keycode(key)?);
                }
            }
        }

        Ok(Shortcut::new(
            key_code,
            if has_mods { Some(mods) } else { None },
        ))
    }

    pub fn from_iced(physical_key: Physical, modifiers: Modifiers) -> Option<Self> {
        let Physical::Code(code) = physical_key else {
            return None;
        };

        let key_code = match code {
            Code::KeyA => 0x00,
            Code::KeyS => 0x01,
            Code::KeyD => 0x02,
            Code::KeyF => 0x03,
            Code::KeyH => 0x04,
            Code::KeyG => 0x05,
            Code::KeyZ => 0x06,
            Code::KeyX => 0x07,
            Code::KeyC => 0x08,
            Code::KeyV => 0x09,
            Code::KeyB => 0x0b,
            Code::KeyQ => 0x0c,
            Code::KeyW => 0x0d,
            Code::KeyE => 0x0e,
            Code::KeyR => 0x0f,
            Code::KeyY => 0x10,
            Code::KeyT => 0x11,
            Code::KeyO => 0x1f,
            Code::KeyU => 0x20,
            Code::KeyI => 0x22,
            Code::KeyP => 0x23,
            Code::KeyL => 0x25,
            Code::KeyJ => 0x26,
            Code::KeyK => 0x28,
            Code::KeyN => 0x2d,
            Code::KeyM => 0x2e,
            Code::Digit1 => 0x12,
            Code::Digit2 => 0x13,
            Code::Digit3 => 0x14,
            Code::Digit4 => 0x15,
            Code::Digit5 => 0x17,
            Code::Digit6 => 0x16,
            Code::Digit7 => 0x1a,
            Code::Digit8 => 0x1c,
            Code::Digit9 => 0x19,
            Code::Digit0 => 0x1d,
            Code::Enter => 0x24,
            Code::Tab => 0x30,
            Code::Space => 0x31,
            Code::Backspace => 0x33,
            Code::Escape => 0x35,
            Code::ArrowLeft => 0x7b,
            Code::ArrowRight => 0x7c,
            Code::ArrowDown => 0x7d,
            Code::ArrowUp => 0x7e,
            Code::Home => 0x73,
            Code::End => 0x77,
            Code::PageUp => 0x74,
            Code::PageDown => 0x79,
            Code::F1 => 0x7a,
            Code::F2 => 0x78,
            Code::F3 => 0x63,
            Code::F4 => 0x76,
            Code::F5 => 0x60,
            Code::F6 => 0x61,
            Code::F7 => 0x62,
            Code::F8 => 0x64,
            Code::F9 => 0x65,
            Code::F10 => 0x6d,
            Code::F11 => 0x67,
            Code::F12 => 0x6f,
            Code::Minus => 0x1b,
            Code::Equal => 0x18,
            Code::BracketLeft => 0x21,
            Code::BracketRight => 0x1e,
            Code::Backslash => 0x2a,
            Code::Semicolon => 0x29,
            Code::Quote => 0x27,
            Code::Backquote => 0x32,
            Code::Comma => 0x2b,
            Code::Period => 0x2f,
            Code::Slash => 0x2c,
            _ => return None,
        };

        let mut mods = NSEventModifierFlags::empty();
        if modifiers.logo() {
            mods |= NSEventModifierFlags::Command;
        }
        if modifiers.alt() {
            mods |= NSEventModifierFlags::Option;
        }
        if modifiers.control() {
            mods |= NSEventModifierFlags::Control;
        }
        if modifiers.shift() {
            mods |= NSEventModifierFlags::Shift;
        }
        if mods.is_empty() {
            return None;
        }

        Some(Self::new(Some(key_code), Some(mods.0)))
    }

    pub fn to_config_string(&self) -> String {
        let mut parts = Vec::new();
        let mods = self.mods.unwrap_or_default();
        if mods & NSEventModifierFlags::Command.0 != 0 {
            parts.push("cmd".to_string());
        }
        if mods & NSEventModifierFlags::Option.0 != 0 {
            parts.push("option".to_string());
        }
        if mods & NSEventModifierFlags::Control.0 != 0 {
            parts.push("ctrl".to_string());
        }
        if mods & NSEventModifierFlags::Shift.0 != 0 {
            parts.push("shift".to_string());
        }
        if let Some(key_code) = self.key_code.and_then(keycode_to_name) {
            parts.push(key_code.to_string());
        }
        parts.join("+")
    }

    pub fn display_string(&self) -> String {
        let mut display = String::new();
        let mods = self.mods.unwrap_or_default();
        if mods & NSEventModifierFlags::Command.0 != 0 {
            display.push('⌘');
        }
        if mods & NSEventModifierFlags::Option.0 != 0 {
            display.push('⌥');
        }
        if mods & NSEventModifierFlags::Control.0 != 0 {
            display.push('⌃');
        }
        if mods & NSEventModifierFlags::Shift.0 != 0 {
            display.push('⇧');
        }
        if let Some(key) = self.key_code.and_then(keycode_to_name) {
            let key = match key {
                "return" => "↩",
                "space" => "Space",
                "delete" => "⌫",
                "escape" => "⎋",
                "left" => "←",
                "right" => "→",
                "down" => "↓",
                "up" => "↑",
                _ => key,
            };
            if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() {
                display.push_str(&key.to_ascii_uppercase());
            } else {
                display.push_str(key);
            }
        }
        display
    }
}

fn keycode_to_name(code: u16) -> Option<&'static str> {
    Some(match code {
        0x00 => "a",
        0x01 => "s",
        0x02 => "d",
        0x03 => "f",
        0x04 => "h",
        0x05 => "g",
        0x06 => "z",
        0x07 => "x",
        0x08 => "c",
        0x09 => "v",
        0x0b => "b",
        0x0c => "q",
        0x0d => "w",
        0x0e => "e",
        0x0f => "r",
        0x10 => "y",
        0x11 => "t",
        0x1f => "o",
        0x20 => "u",
        0x22 => "i",
        0x23 => "p",
        0x25 => "l",
        0x26 => "j",
        0x28 => "k",
        0x2d => "n",
        0x2e => "m",
        0x12 => "1",
        0x13 => "2",
        0x14 => "3",
        0x15 => "4",
        0x17 => "5",
        0x16 => "6",
        0x1a => "7",
        0x1c => "8",
        0x19 => "9",
        0x1d => "0",
        0x24 => "return",
        0x30 => "tab",
        0x31 => "space",
        0x33 => "delete",
        0x35 => "escape",
        0x7b => "left",
        0x7c => "right",
        0x7d => "down",
        0x7e => "up",
        0x73 => "home",
        0x77 => "end",
        0x74 => "pageup",
        0x79 => "pagedown",
        0x7a => "f1",
        0x78 => "f2",
        0x63 => "f3",
        0x76 => "f4",
        0x60 => "f5",
        0x61 => "f6",
        0x62 => "f7",
        0x64 => "f8",
        0x65 => "f9",
        0x6d => "f10",
        0x67 => "f11",
        0x6f => "f12",
        0x1b => "-",
        0x18 => "=",
        0x21 => "[",
        0x1e => "]",
        0x2a => "\\",
        0x29 => ";",
        0x27 => "'",
        0x32 => "`",
        0x2b => ",",
        0x2f => ".",
        0x2c => "/",
        _ => return None,
    })
}

fn str_to_keycode(s: &str) -> Result<u16, String> {
    let code = match s.to_lowercase().as_str() {
        // Letters
        "a" => 0x00,
        "s" => 0x01,
        "d" => 0x02,
        "f" => 0x03,
        "h" => 0x04,
        "g" => 0x05,
        "z" => 0x06,
        "x" => 0x07,
        "c" => 0x08,
        "v" => 0x09,
        "b" => 0x0b,
        "q" => 0x0c,
        "w" => 0x0d,
        "e" => 0x0e,
        "r" => 0x0f,
        "y" => 0x10,
        "t" => 0x11,
        "o" => 0x1f,
        "u" => 0x20,
        "i" => 0x22,
        "p" => 0x23,
        "l" => 0x25,
        "j" => 0x26,
        "k" => 0x28,
        "n" => 0x2d,
        "m" => 0x2e,

        // Numbers
        "1" => 0x12,
        "2" => 0x13,
        "3" => 0x14,
        "4" => 0x15,
        "5" => 0x17,
        "6" => 0x16,
        "7" => 0x1a,
        "8" => 0x1c,
        "9" => 0x19,
        "0" => 0x1d,

        // Special keys
        "return" | "enter" => 0x24,
        "tab" => 0x30,
        "space" => 0x31,
        "delete" | "backspace" => 0x33,
        "escape" | "esc" => 0x35,
        "left" | "arrowleft" => 0x7b,
        "right" | "arrowright" => 0x7c,
        "down" | "arrowdown" => 0x7d,
        "up" | "arrowup" => 0x7e,
        "home" => 0x73,
        "end" => 0x77,
        "pageup" => 0x74,
        "pagedown" => 0x79,

        // Function keys
        "f1" => 0x7a,
        "f2" => 0x78,
        "f3" => 0x63,
        "f4" => 0x76,
        "f5" => 0x60,
        "f6" => 0x61,
        "f7" => 0x62,
        "f8" => 0x64,
        "f9" => 0x65,
        "f10" => 0x6d,
        "f11" => 0x67,
        "f12" => 0x6f,

        // Symbols
        "-" | "minus" => 0x1b,
        "=" | "equal" => 0x18,
        "[" | "bracketleft" => 0x21,
        "]" | "bracketright" => 0x1e,
        "\\" | "backslash" => 0x2a,
        ";" | "semicolon" => 0x29,
        "'" | "quote" => 0x27,
        "`" | "backquote" | "grave" => 0x32,
        "," | "comma" => 0x2b,
        "." | "period" => 0x2f,
        "/" | "slash" => 0x2c,
        _ => return Err(format!("Unknown key: '{}'", s)),
    };

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_shortcut_uses_config_format() -> Result<(), String> {
        let shortcut = Shortcut::from_iced(
            Physical::Code(Code::KeyK),
            Modifiers::LOGO | Modifiers::SHIFT,
        )
        .ok_or_else(|| "modified shortcut should be recorded".to_string())?;

        assert_eq!(shortcut.to_config_string(), "cmd+shift+k");
        assert_eq!(shortcut.display_string(), "⌘⇧K");
        Ok(())
    }

    #[test]
    fn recorded_shortcut_requires_a_modifier() {
        assert!(Shortcut::from_iced(Physical::Code(Code::KeyK), Modifiers::NONE).is_none());
    }
}
