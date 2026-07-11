use std::{ffi::c_void, ptr::NonNull};

use objc2_core_foundation::{CFArray, CFRetained, CFString, CFType};

type Boolean = u8;
type OSStatus = i32;
type TISInputSourceRef = *const c_void;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSource {
    pub id: String,
    pub name: String,
}

impl std::fmt::Display for InputSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyInputSourceID: *const CFString;
    static kTISPropertyLocalizedName: *const CFString;

    fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    fn TISCreateInputSourceList(
        properties: *const c_void,
        includeAllInstalled: Boolean,
    ) -> *const CFArray;
    fn TISGetInputSourceProperty(
        inputSource: TISInputSourceRef,
        propertyKey: *const CFString,
    ) -> *const c_void;
    fn TISSelectInputSource(inputSource: TISInputSourceRef) -> OSStatus;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *mut CFType);
}

pub fn current_input_source_id() -> Option<String> {
    let source = unsafe { TISCopyCurrentKeyboardInputSource() };
    if source.is_null() {
        return None;
    }

    let id = input_source_id(source);
    unsafe { CFRelease(source.cast_mut().cast()) };
    id
}

pub fn select_input_source(id: &str) -> Result<(), String> {
    let Some(sources) = input_source_list(false) else {
        return Err("TISCreateInputSourceList returned null".to_string());
    };

    select_input_source_from_list(&sources, id)
}

pub fn enabled_input_sources() -> Vec<InputSource> {
    input_source_list(false)
        .map(|sources| input_sources_from_list(&sources))
        .unwrap_or_default()
}

fn input_source_list(include_all_installed: bool) -> Option<CFRetained<CFArray>> {
    let sources =
        unsafe { TISCreateInputSourceList(std::ptr::null(), include_all_installed as Boolean) };
    NonNull::new(sources.cast_mut()).map(|sources| unsafe { CFRetained::from_raw(sources) })
}

fn select_input_source_from_list(sources: &CFArray, id: &str) -> Result<(), String> {
    for idx in 0..sources.count() {
        let source = unsafe { sources.value_at_index(idx) };
        if source.is_null() {
            continue;
        }

        if input_source_id(source).as_deref() == Some(id) {
            let status = unsafe { TISSelectInputSource(source) };
            return if status == 0 {
                Ok(())
            } else {
                Err(format!("TISSelectInputSource failed with status {status}"))
            };
        }
    }

    Err(format!("input source not found: {id}"))
}

fn input_sources_from_list(sources: &CFArray) -> Vec<InputSource> {
    (0..sources.count())
        .filter_map(|idx| {
            let source = unsafe { sources.value_at_index(idx) };
            if source.is_null() {
                return None;
            }
            let id = input_source_id(source)?;
            let name = input_source_name(source).unwrap_or_else(|| id.clone());
            Some(InputSource { id, name })
        })
        .collect()
}

fn input_source_id(source: TISInputSourceRef) -> Option<String> {
    let id_ref = unsafe { TISGetInputSourceProperty(source, kTISPropertyInputSourceID) };
    if id_ref.is_null() {
        return None;
    }

    cf_string_to_string(id_ref)
}

fn input_source_name(source: TISInputSourceRef) -> Option<String> {
    let name_ref = unsafe { TISGetInputSourceProperty(source, kTISPropertyLocalizedName) };
    if name_ref.is_null() {
        return None;
    }

    cf_string_to_string(name_ref)
}

fn cf_string_to_string(value: *const c_void) -> Option<String> {
    NonNull::new(value.cast_mut().cast::<CFString>())
        .map(|value| unsafe { value.as_ref() }.to_string())
}
