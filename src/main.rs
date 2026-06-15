#![allow(clippy::single_match)]
mod bluetooth;
mod cli_args;
mod config;
mod error;
mod general_options;
mod lua_decoder;
mod route;
mod tui;

use crate::{bluetooth::BleScan, config::Config, tui::run_tui_app};
use btleplug::platform::Manager;
use clap::Parser;
use cli_args::Args;
use general_options::GeneralOptions;
use std::env;
use std::sync::RwLock;
use std::sync::{Arc, Mutex, RwLockReadGuard};

#[derive(Debug)]
pub struct Ctx {
    args: Args,
    /// Parsed TOML config (custom names + Lua decoders). Empty when no
    /// `--config` file was provided.
    config: Config,
    ble_manager: Manager,
    latest_scan: RwLock<Option<BleScan>>,
    active_route: RwLock<route::Route>,
    active_side_effect_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    request_scan_restart: Mutex<bool>,
    global_error: Mutex<Option<crate::error::Error>>,
    general_options: RwLock<general_options::GeneralOptions>,
}

impl Ctx {
    pub fn get_active_route(&self) -> RwLockReadGuard<'_, route::Route> {
        self.active_route
            .read()
            // This should be generally safe to unwrap here because we do have only write lock and we can not use tokio's rwlock
            // because we must read it sync for rendering
            .expect("Failed to acquire active route lock.")
    }

    /// Resolve a user-provided custom name for a UUID, checking the
    /// `--names-map-file` .ini map first and then the TOML config's
    /// per-characteristic `name`.
    pub fn custom_name(&self, uuid: &uuid::Uuid) -> Option<String> {
        self.args
            .names_map_file
            .as_ref()
            .and_then(|names| names.get(uuid).cloned())
            .or_else(|| {
                self.config
                    .characteristics
                    .get(uuid)
                    .and_then(|c| c.name.clone())
            })
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let file_appender = tracing_appender::rolling::daily(env::temp_dir().join("blendr"), "cli.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_max_level(match args.log_level.unwrap_or_default() {
            cli_args::LogLevel::Debug => tracing::Level::DEBUG,
            cli_args::LogLevel::Error => tracing::Level::ERROR,
        })
        .pretty()
        .init();

    // Load the TOML config (custom names + Lua decoders): an explicit --config,
    // else an auto-discovered .blendr.toml in the current directory, else empty.
    // A malformed config is a hard error: fail fast with a readable message
    // rather than silently dropping the user's decoders.
    let config = match Config::resolve(args.config.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let ctx = Arc::new(Ctx {
        latest_scan: RwLock::new(None),
        active_route: RwLock::new(route::Route::PeripheralList),
        active_side_effect_handle: Mutex::new(None),
        ble_manager: Manager::new()
            .await
            .expect("Can not establish BLE connection."),
        request_scan_restart: Mutex::new(false),
        global_error: Mutex::new(None),
        general_options: RwLock::new(GeneralOptions::new(&args)),
        config,
        args,
    });

    let ctx_clone = Arc::clone(&ctx);
    let _scanner = tokio::spawn(async move {
        if let Err(e) = bluetooth::start_scan(Arc::clone(&ctx_clone)).await {
            ctx_clone.global_error.lock().unwrap().replace(e);
        }
    });

    run_tui_app(ctx).unwrap();
}
