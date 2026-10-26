//! TOML configuration file support.
//!
//! Unlike the simple `--names-map-file` .ini format (UUID=NAME pairs only), the
//! TOML config attaches per-characteristic settings keyed by UUID: a custom
//! display `name` and/or a Lua `script` that decodes incoming values.
//!
//! ```toml
//! # One entry per characteristic, keyed by its UUID.
//! [characteristics."0000ffe1-0000-1000-8000-00805f9b34fb"]
//! name = "Core Service Data"
//!
//! [characteristics."5f78df94-798c-46f5-990a-b3eb6a065c88"]
//! name = "Temperature"
//! # `script` is either a path to a .lua file (relative to this config) ...
//! script = "decoders/temperature.lua"
//!
//! [characteristics."61c8849c-f639-4765-946e-5c3419bebb2a"]
//! name = "Duration"
//! # ... or an inline Lua snippet.
//! script = "return string.format('%.3f s', read('<I4', 0) / 1000)"
//! ```
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Per-characteristic configuration, keyed by UUID in [`Config::characteristics`].
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CharacteristicConfig {
    /// Human friendly label shown in the UI (defaults to the UUID).
    #[serde(default)]
    pub name: Option<String>,
    /// Lua decoder. In the file this is either a path to a `.lua` file or an
    /// inline snippet; after [`Config::load`] it always holds inline source.
    #[serde(default)]
    pub script: Option<String>,
}

impl CharacteristicConfig {
    /// Label used in the UI and in Lua error messages.
    pub fn display_name(&self, uuid: &uuid::Uuid) -> String {
        self.name.clone().unwrap_or_else(|| uuid.to_string())
    }
}

/// Config file name auto-discovered in the current directory when `--config`
/// is not given.
pub const DEFAULT_CONFIG_FILE: &str = ".blendr.toml";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// Per-characteristic settings (custom names + Lua decoders), keyed by UUID.
    #[serde(default)]
    pub characteristics: HashMap<uuid::Uuid, CharacteristicConfig>,
}

/// Resolve a `script` value to a `.lua` file path when it both ends in `.lua`
/// and points at an existing file (relative to the config's directory).
/// Otherwise the value is treated as inline Lua source. Returns `None` for
/// inline scripts, `Some(path)` for file-backed ones.
fn resolve_script_path(value: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let trimmed = value.trim();
    if !trimmed.ends_with(".lua") || trimmed.contains('\n') {
        return None;
    }

    let path = match base_dir {
        Some(dir) => dir.join(trimmed),
        None => PathBuf::from(trimmed),
    };

    path.is_file().then_some(path)
}

impl Config {
    /// True if any characteristic defines a Lua decoder.
    pub fn has_decoders(&self) -> bool {
        self.characteristics.values().any(|c| c.script.is_some())
    }

    /// Resolve the effective config given the optional `--config` argument,
    /// auto-discovering `.blendr.toml` in the current directory. See
    /// [`Config::resolve_in`] for the precedence rules.
    pub fn resolve(explicit: Option<&Path>) -> Result<Config, String> {
        Config::resolve_in(explicit, Path::new("."))
    }

    /// Resolve the effective config, looking for an auto-discovered
    /// `.blendr.toml` inside `dir`:
    ///
    /// 1. an explicit `--config` path is always loaded (and a failure to parse
    ///    it is propagated as an error);
    /// 2. otherwise, if a `.blendr.toml` exists in `dir`, it is auto-loaded (a
    ///    parse failure there is still an error, since a config is clearly
    ///    intended);
    /// 3. otherwise an empty default config is returned.
    pub fn resolve_in(explicit: Option<&Path>, dir: &Path) -> Result<Config, String> {
        if let Some(path) = explicit {
            return Config::load(path);
        }

        let default_path = dir.join(DEFAULT_CONFIG_FILE);
        if default_path.is_file() {
            tracing::debug!("Auto-loading config from {}", default_path.display());
            return Config::load(&default_path);
        }

        Ok(Config::default())
    }

    /// Parse a TOML config file, resolving any `script` that points at an
    /// existing `.lua` file into inline source so the rest of the app only
    /// deals with ready-to-run code. A `script` is loaded from disk only when
    /// it both ends in `.lua` and exists (relative to the config's directory);
    /// anything else is kept as inline Lua.
    pub fn load(path: &Path) -> Result<Config, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {}: {e}", path.display()))?;

        let mut config: Config = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse config file {}:\n{e}", path.display()))?;

        let base_dir = path.parent();
        for (uuid, characteristic) in config.characteristics.iter_mut() {
            let Some(script) = characteristic.script.as_ref() else {
                continue;
            };

            if let Some(script_path) = resolve_script_path(script, base_dir) {
                let source = std::fs::read_to_string(&script_path).map_err(|e| {
                    format!(
                        "Failed to read Lua script {} for characteristic {uuid}: {e}",
                        script_path.display()
                    )
                })?;

                characteristic.script = Some(source);
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_decoder::{DecodeOutcome, LuaDecoderEngine};

    /// Load the example config shipped in the repo, resolve its external `.lua`
    /// script, then run a decoder through the real Lua engine. Exercises the
    /// whole pipeline: TOML parse -> file resolution -> decode.
    #[test]
    fn example_config_parses_and_decodes() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config_example.toml");
        let config = Config::load(&path).expect("example config should load");

        assert!(config.has_decoders());

        let engine = LuaDecoderEngine::new(&config).expect("engine builds");

        // Temperature decoder: <i2 / 100. 0x0929 LE = 2345 -> "23.45 °C".
        let temp_uuid = uuid::Uuid::parse_str("5f78df94-798c-46f5-990a-b3eb6a065c88").unwrap();
        match engine.decode(&temp_uuid, &[0x29, 0x09]).unwrap() {
            DecodeOutcome::Decoded { value, .. } => assert_eq!(value, "23.45 °C"),
            DecodeOutcome::Error { message, .. } => panic!("temperature decode failed: {message}"),
        }
    }

    #[test]
    fn resolve_script_path_requires_lua_suffix_and_existence() {
        let dir = std::env::temp_dir().join("blendr_config_test");
        std::fs::create_dir_all(&dir).unwrap();
        let existing = dir.join("decoder.lua");
        std::fs::write(&existing, "return 1").unwrap();

        // .lua suffix AND exists -> treated as a path.
        assert_eq!(
            resolve_script_path("decoder.lua", Some(&dir)),
            Some(existing.clone())
        );
        // .lua suffix but does NOT exist -> inline (None).
        assert_eq!(resolve_script_path("missing.lua", Some(&dir)), None);
        // Looks like Lua source that happens to mention .lua -> inline.
        assert_eq!(resolve_script_path("return read('<i2', 0)", Some(&dir)), None);
        // Multi-line is always inline even if it ends in .lua.
        assert_eq!(resolve_script_path("return x\n-- a.lua", Some(&dir)), None);

        std::fs::remove_file(&existing).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn nonexistent_lua_path_is_kept_as_inline_source() {
        // A .lua value that doesn't resolve to a file is left untouched as
        // inline source rather than raising a "file not found" error.
        let toml = r#"
            [characteristics."5f78df94-798c-46f5-990a-b3eb6a065c88"]
            script = "does-not-exist.lua"
        "#;
        let dir = std::env::temp_dir().join("blendr_config_inline_test");
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("config.toml");
        std::fs::write(&config_path, toml).unwrap();

        let config = Config::load(&config_path).expect("should load, treating value as inline");
        let uuid = uuid::Uuid::parse_str("5f78df94-798c-46f5-990a-b3eb6a065c88").unwrap();
        assert_eq!(
            config.characteristics.get(&uuid).unwrap().script.as_deref(),
            Some("does-not-exist.lua")
        );

        std::fs::remove_file(&config_path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn parses_uuid_keyed_table() {
        let toml = r#"
            [characteristics."5f78df94-798c-46f5-990a-b3eb6a065c88"]
            name = "Temperature"
            script = "return 1 + 1"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        let uuid = uuid::Uuid::parse_str("5f78df94-798c-46f5-990a-b3eb6a065c88").unwrap();
        let entry = config.characteristics.get(&uuid).unwrap();
        assert_eq!(entry.name.as_deref(), Some("Temperature"));
        assert_eq!(entry.script.as_deref(), Some("return 1 + 1"));
    }

    /// Create a uniquely-named temp dir for a discovery test, returning its path.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("blendr_resolve_{tag}"));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn auto_discovers_blendr_toml_in_dir() {
        let dir = temp_dir("autodiscover");
        std::fs::write(
            dir.join(DEFAULT_CONFIG_FILE),
            "[characteristics.\"5f78df94-798c-46f5-990a-b3eb6a065c88\"]\nname = \"Auto\"\n",
        )
        .unwrap();

        let config = Config::resolve_in(None, &dir).expect("auto-discovered config loads");
        let uuid = uuid::Uuid::parse_str("5f78df94-798c-46f5-990a-b3eb6a065c88").unwrap();
        assert_eq!(
            config.characteristics.get(&uuid).and_then(|c| c.name.as_deref()),
            Some("Auto")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_config_when_no_file_present() {
        let dir = temp_dir("nofile");
        let config = Config::resolve_in(None, &dir).expect("missing file is not an error");
        assert!(config.characteristics.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn explicit_config_takes_precedence_over_auto_discovery() {
        let dir = temp_dir("precedence");
        // A .blendr.toml that should be ignored when --config is given.
        std::fs::write(
            dir.join(DEFAULT_CONFIG_FILE),
            "[characteristics.\"5f78df94-798c-46f5-990a-b3eb6a065c88\"]\nname = \"FromAuto\"\n",
        )
        .unwrap();
        let explicit = dir.join("explicit.toml");
        std::fs::write(
            &explicit,
            "[characteristics.\"5f78df94-798c-46f5-990a-b3eb6a065c88\"]\nname = \"FromExplicit\"\n",
        )
        .unwrap();

        let config = Config::resolve_in(Some(&explicit), &dir).expect("explicit config loads");
        let uuid = uuid::Uuid::parse_str("5f78df94-798c-46f5-990a-b3eb6a065c88").unwrap();
        assert_eq!(
            config.characteristics.get(&uuid).and_then(|c| c.name.as_deref()),
            Some("FromExplicit")
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
