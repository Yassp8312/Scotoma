//! Audited credential ingress kept separate from generated FRB serialization.
//!
//! Native callers provide at most 128 bytes per field and overwrite their own
//! fixed allocations after the synchronous call. On Wasm, lengths are checked
//! while values are still JS references, then copied into fixed Rust buffers.

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scotoma_secure_credential_stage(
    room_ptr: *const u8,
    room_len: usize,
    password_ptr: *const u8,
    password_len: usize,
) -> u32 {
    use scotoma_protocol::MAX_NUMERIC_DIGITS;

    if room_ptr.is_null()
        || password_ptr.is_null()
        || room_len == 0
        || password_len == 0
        || room_len > MAX_NUMERIC_DIGITS
        || password_len > MAX_NUMERIC_DIGITS
    {
        return 0;
    }
    // SAFETY: the Dart adapter owns both fixed 128-byte allocations for this
    // synchronous call. Lengths were bounded above before creating the slices.
    let (room, password) = unsafe {
        (
            std::slice::from_raw_parts(room_ptr, room_len),
            std::slice::from_raw_parts(password_ptr, password_len),
        )
    };
    crate::api::session::stage_borrowed(room, password)
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn scotoma_secure_message_stage(
    message_ptr: *const u8,
    message_len: usize,
) -> u32 {
    use crate::api::session::MAX_MESSAGE_BYTES;

    if message_ptr.is_null() || message_len == 0 || message_len > MAX_MESSAGE_BYTES {
        return 0;
    }
    // SAFETY: the Dart adapter owns this bounded allocation for the duration
    // of the synchronous call and overwrites it before freeing.
    let message = unsafe { std::slice::from_raw_parts(message_ptr, message_len) };
    crate::api::session::stage_message_borrowed(message)
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web {
    use js_sys::Uint8Array;
    use scotoma_protocol::MAX_NUMERIC_DIGITS;
    use wasm_bindgen::{JsCast, prelude::*};
    use zeroize::Zeroizing;

    #[wasm_bindgen(js_name = scotomaSecureMessageStage)]
    pub fn stage_message(message: JsValue) -> u32 {
        use crate::api::session::MAX_MESSAGE_BYTES;

        if !message.is_instance_of::<Uint8Array>() {
            return 0;
        }
        let message: Uint8Array = message.unchecked_into();
        let length = message.length() as usize;
        if length == 0 || length > MAX_MESSAGE_BYTES {
            return 0;
        }
        let mut bytes = Zeroizing::new(vec![0_u8; length]);
        message.copy_to(&mut bytes);
        crate::api::session::stage_message_borrowed(&bytes)
    }

    #[wasm_bindgen(js_name = scotomaSecureCredentialStage)]
    pub fn stage(room: JsValue, password: JsValue) -> u32 {
        if !room.is_instance_of::<Uint8Array>() || !password.is_instance_of::<Uint8Array>() {
            return 0;
        }
        let room: Uint8Array = room.unchecked_into();
        let password: Uint8Array = password.unchecked_into();
        let room_len = room.length() as usize;
        let password_len = password.length() as usize;
        if room_len == 0
            || password_len == 0
            || room_len > MAX_NUMERIC_DIGITS
            || password_len > MAX_NUMERIC_DIGITS
        {
            return 0;
        }
        let mut room_bytes = Zeroizing::new([0u8; MAX_NUMERIC_DIGITS]);
        let mut password_bytes = Zeroizing::new([0u8; MAX_NUMERIC_DIGITS]);
        room.copy_to(&mut room_bytes[..room_len]);
        password.copy_to(&mut password_bytes[..password_len]);
        crate::api::session::stage_borrowed(
            &room_bytes[..room_len],
            &password_bytes[..password_len],
        )
    }
}
