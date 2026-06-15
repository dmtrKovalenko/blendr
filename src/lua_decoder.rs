//! Custom Lua decoding for incoming characteristic values.
//!
//! Each decoder is a small Lua script bound to a characteristic UUID. When a
//! value arrives, the script runs with the raw bytes exposed through a few
//! globals and is expected to `return` a string for display.
//!
//! ## Globals available to a script
//!
//! - `data`  : the raw value as a Lua string (use with `string.unpack`)
//! - `bytes` : a 1-indexed table of byte integers (`bytes[1]` is the first byte)
//! - `len`   : number of bytes
//! - `hex`   : lowercase hex representation, e.g. `"a1b2c3"`
//! - `read(fmt, offset)` : convenience wrapper over `string.unpack`. `offset`
//!   is 0-indexed and defaults to 0. Returns the first unpacked value, so
//!   `read("<i2", 0)` reads a little-endian 16-bit signed int at the start.
//!
//! ## Example
//!
//! ```lua
//! -- temperature stored as little-endian centidegrees
//! return string.format("%.2f °C", read("<i2", 0) / 100)
//! ```
//!
//! The engine is intentionally single-threaded: it lives inside the TUI's
//! `ConnectionView` and is only ever touched from the render thread, so the
//! non-`Send` `mlua::Lua` never crosses a thread boundary.

use crate::config::Config;
use mlua::{Lua, MultiValue, Value};
use std::collections::HashMap;

/// One compiled decoder ready to run against incoming bytes.
struct CompiledDecoder {
    name: String,
    /// The decoder's Lua source. We re-load it per call into a freshly scoped
    /// environment; compilation cost is negligible next to the BLE poll cadence
    /// and it keeps each invocation free of leftover global state.
    source: String,
}

/// Owns the Lua VM and the set of per-UUID decoders.
///
/// Not `Send`/`Sync` — by design. Construct and use it from a single thread.
pub struct LuaDecoderEngine {
    lua: Lua,
    decoders: HashMap<uuid::Uuid, CompiledDecoder>,
}

/// Outcome of decoding a value with a custom script.
pub enum DecodeOutcome {
    /// Script ran and produced a display string.
    Decoded { name: String, value: String },
    /// Script raised an error (syntax or runtime).
    Error { name: String, message: String },
}

impl LuaDecoderEngine {
    /// Build an engine from the configured characteristics. Only those with a
    /// `script` become decoders. Each script is compile-checked once up front;
    /// scripts that fail to compile are reported but do not abort startup (the
    /// rest still load).
    pub fn new(config: &Config) -> mlua::Result<Self> {
        let lua = Lua::new();
        Self::install_helpers(&lua)?;

        let mut decoders = HashMap::new();
        for (uuid, characteristic) in config.characteristics.iter() {
            let Some(source) = characteristic.script.clone() else {
                continue;
            };
            let name = characteristic.display_name(uuid);

            // Compile-check so obvious syntax errors surface immediately rather
            // than on the first incoming value.
            if let Err(e) = lua.load(&source).into_function() {
                tracing::error!(uuid = %uuid, name = %name, "Failed to compile Lua decoder: {e}");
            }

            decoders.insert(*uuid, CompiledDecoder { name, source });
        }

        Ok(LuaDecoderEngine { lua, decoders })
    }

    /// Run the decoder bound to `uuid` against `data`. Returns `None` when no
    /// decoder is registered for that characteristic.
    pub fn decode(&self, uuid: &uuid::Uuid, data: &[u8]) -> Option<DecodeOutcome> {
        let decoder = self.decoders.get(uuid)?;

        Some(match self.run(decoder, data) {
            Ok(value) => DecodeOutcome::Decoded {
                name: decoder.name.clone(),
                value,
            },
            Err(e) => DecodeOutcome::Error {
                name: decoder.name.clone(),
                message: e.to_string(),
            },
        })
    }

    /// Install globals derived from the current value, then execute the script.
    fn run(&self, decoder: &CompiledDecoder, data: &[u8]) -> mlua::Result<String> {
        let globals = self.lua.globals();

        // `data` as a binary Lua string for use with string.unpack.
        globals.set("data", self.lua.create_string(data)?)?;
        globals.set("len", data.len())?;
        globals.set("hex", hex_string(data))?;

        let bytes = self.lua.create_table()?;
        for (i, b) in data.iter().enumerate() {
            bytes.set(i + 1, *b)?; // 1-indexed to match Lua conventions
        }
        globals.set("bytes", bytes)?;

        let chunk = self
            .lua
            .load(&decoder.source)
            .set_name(decoder.name.clone());

        let result: Value = chunk.eval()?;
        Ok(value_to_display(&result))
    }

    /// Install helper functions shared by every decoder.
    fn install_helpers(lua: &Lua) -> mlua::Result<()> {
        // read(fmt, offset?) -> first value unpacked by string.unpack.
        // offset is 0-indexed; string.unpack positions are 1-indexed.
        let read = lua.create_function(|lua, (fmt, offset): (mlua::String, Option<usize>)| {
            let globals = lua.globals();
            let data: mlua::String = globals.get("data")?;
            let string_table: mlua::Table = globals.get("string")?;
            let unpack: mlua::Function = string_table.get("unpack")?;

            let pos = offset.unwrap_or(0) + 1;
            let results: MultiValue = unpack.call((fmt, data, pos))?;
            // string.unpack returns (values..., next_position); hand back the
            // first unpacked value, which is what a one-field format wants.
            Ok(results.into_iter().next().unwrap_or(Value::Nil))
        })?;

        lua.globals().set("read", read)?;
        Ok(())
    }
}

/// Render a returned Lua value as a display string. Strings and numbers map
/// directly; anything else falls back to a type-tagged debug form.
fn value_to_display(value: &Value) -> String {
    match value {
        Value::String(s) => s.to_string_lossy().to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Nil => "nil".to_string(),
        other => format!("{other:?}"),
    }
}

fn hex_string(data: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::CharacteristicConfig;

    const TEST_UUID: u128 = 0x0000FFE0_0000_1000_8000_00805F9B34FB;

    fn config_with(uuid: uuid::Uuid, script: &str) -> Config {
        let mut characteristics = HashMap::new();
        characteristics.insert(
            uuid,
            CharacteristicConfig {
                name: Some("test".to_string()),
                script: Some(script.to_string()),
            },
        );
        Config { characteristics }
    }

    fn decode(script: &str, data: &[u8]) -> DecodeOutcome {
        let uuid = uuid::Uuid::from_u128(TEST_UUID);
        let engine = LuaDecoderEngine::new(&config_with(uuid, script)).unwrap();
        engine.decode(&uuid, data).expect("decoder should be registered")
    }

    fn decoded_value(script: &str, data: &[u8]) -> String {
        match decode(script, data) {
            DecodeOutcome::Decoded { value, .. } => value,
            DecodeOutcome::Error { message, .. } => panic!("decoder errored: {message}"),
        }
    }

    #[test]
    fn returns_none_for_unregistered_uuid() {
        let registered = uuid::Uuid::from_u128(TEST_UUID);
        let other = uuid::Uuid::from_u128(0x0000FFE1_0000_1000_8000_00805F9B34FB);
        let engine = LuaDecoderEngine::new(&config_with(registered, "return 'x'")).unwrap();
        assert!(engine.decode(&other, &[1, 2, 3]).is_none());
    }

    #[test]
    fn exposes_len_and_bytes_globals() {
        assert_eq!(decoded_value("return len", &[10, 20, 30]), "3");
        // bytes is 1-indexed
        assert_eq!(decoded_value("return bytes[1]", &[10, 20, 30]), "10");
        assert_eq!(decoded_value("return bytes[3]", &[10, 20, 30]), "30");
    }

    #[test]
    fn exposes_hex_global() {
        assert_eq!(decoded_value("return hex", &[0xa1, 0xb2, 0xc3]), "a1b2c3");
    }

    #[test]
    fn read_helper_unpacks_little_endian_int() {
        // 0x0100 little-endian = 256
        assert_eq!(decoded_value("return read('<i2', 0)", &[0x00, 0x01]), "256");
    }

    #[test]
    fn read_helper_respects_offset() {
        // skip first byte, read next u8
        assert_eq!(decoded_value("return read('B', 1)", &[0xff, 0x2a]), "42");
    }

    #[test]
    fn data_global_works_with_string_unpack() {
        // little-endian f32 1.5 = 00 00 C0 3F
        let value = decoded_value(
            "return string.format('%.1f', string.unpack('<f', data))",
            &[0x00, 0x00, 0xC0, 0x3F],
        );
        assert_eq!(value, "1.5");
    }

    #[test]
    fn runtime_error_is_captured_not_panicked() {
        match decode("error('boom')", &[1]) {
            DecodeOutcome::Error { message, .. } => assert!(message.contains("boom")),
            DecodeOutcome::Decoded { value, .. } => panic!("expected error, got {value}"),
        }
    }

    #[test]
    fn numeric_return_is_stringified() {
        assert_eq!(decoded_value("return 1 + 2", &[]), "3");
    }
}
