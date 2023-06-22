#![allow(clippy::single_match)]
mod bluetooth;
mod error;
mod route;
mod tui;
use btleplug::platform::Manager;
use clap::Parser;
use std::sync::RwLock;
use std::sync::{Arc, Mutex, RwLockReadGuard};

use crate::{bluetooth::BleScan, tui::run_tui_app};

#[derive(Debug, Parser)]
#[command(
    version=env!("CARGO_PKG_VERSION"),
    author = "Dmitriy Kovalenko <dmtr.kovalenko@outlook.com>", 
    about="vim-style BLE browser terminal client",
    long_about="Blendr is a BLE browser terminal library. It allows to search for BLE peripherals, establish connections, interact with their services and characteristics, and read and write data right from your terminal."
)]
struct Args {
    #[clap(long, short)]
    /// Bluetooth adapter hardware index to use, if many available.
    /// If not specified, the first discovered adapter will be used.
    #[clap(default_value_t = 0)]
    adapter_index: usize,

    #[clap(long, short)]
    /// Scan interval in milliseconds.
    #[clap(default_value_t = 1000)]
    scan_interval: u64,

    #[clap(long, short)]
    #[clap(default_value_t = String::from("(?i)"))]
    /// Regex flags that by default applier to the filter queries.
    /// By default contains case-insensitive flag (?i). Pass --regex-flags "" to make searches case sensitive.
    regex_flags: String,

    #[clap(short, long)]
    /// Device name to search for on start. If only one device would be found matching this filter it will be connected automatically.
    device: Option<String>,

    #[clap(short, long)]
    /// Characteristic or service uui search that will be applied on start. If one characteristic will be found matching this filter it will be selected automatically.
    characteristic: Option<String>,
}

#[derive(Debug)]
pub struct Ctx {
    args: Args,
    ble_manager: Manager,
    latest_scan: RwLock<Option<BleScan>>,
    active_route: RwLock<route::Route>,
    active_side_effect_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Ctx {
    pub fn get_active_route(&self) -> RwLockReadGuard<'_, route::Route> {
        self.active_route
            .read()
            // This should be generally safe to unwrap here because we do have only write lock and we can not use tokio's rwlock
            // because we must read it sync for rendering
            .expect("Failed to acquire active route lock.")
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let ctx = Arc::new(Ctx {
        args,
        latest_scan: RwLock::new(None),
        active_route: RwLock::new(route::Route::PeripheralList),
        active_side_effect_handle: Mutex::new(None),
        ble_manager: Manager::new()
            .await
            .expect("Can not estabilish BLE connection."),
    });

    let _scanner = tokio::spawn(bluetooth::start_scan(Arc::clone(&ctx)));

    run_tui_app(ctx).unwrap();
}
