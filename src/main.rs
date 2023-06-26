#![allow(clippy::single_match)]
mod bluetooth;
mod error;
mod route;
mod tui;
use btleplug::platform::Manager;
use clap::Parser;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::{Arc, Mutex, RwLockReadGuard};

use crate::{bluetooth::BleScan, tui::run_tui_app};

fn parse_name_map(path: &str) -> Result<HashMap<uuid::Uuid, String>, clap::Error> {
    let path = std::path::Path::new(path);

    if !path.exists() || !path.is_file() {
        return Err(clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            format!("File {} does not exist.", path.display()),
        ));
    }

    let content = std::fs::read_to_string(path)?;

    content
        .trim()
        .split('\n')
        .enumerate()
        .map(|(i, line)| -> clap::error::Result<_> {
            let [uuid, name]: [&str; 2] =
                line.split('=')
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| {
                        clap::Error::raw(
                            clap::error::ErrorKind::InvalidValue,
                            format!("Failed to parse line {i} from file {}: Missing = in the line. Names ini file supports very simple key-value pairs format where first value is uuid of service or characteristic and the second is the name.\n\ne.g. 0000FFE0-0000-1000-8000-00805F9B34FB=Cpu Tempreture", path.display()),
                        )
                    })?;

            let uuid = uuid::Uuid::parse_str(uuid).map_err(|e| {
                clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    format!(
                        "Failed to parse uuid on the left side of the line {i} from file {}: {e}",
                        path.display()
                    ),
                )
            })?;
            Ok((uuid, name.trim().to_owned()))
        })
        .collect::<clap::error::Result<HashMap<uuid::Uuid, String>>>()
}

#[test]
fn test_parse_name_map() {
    let test_path = std::path::Path::new("test.ini");
    std::fs::write(test_path, "0000FFE0-0000-1000-8000-00805F9B34FB=test data")
        .expect("Unable to write file");

    assert_eq!(
        parse_name_map(
            test_path
                .to_str()
                .expect("Unable to locate path of test .ini file")
        )
        .unwrap(),
        HashMap::from([(
            uuid::Uuid::from_u128(0x0000FFE0_0000_1000_8000_00805F9B34FB),
            "test data".to_string()
        )])
    );

    std::fs::remove_file(test_path).expect("Unable to delete file");
}

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

    /// Device name to search for on start. If only one device would be found matching this filter it will be connected automatically.
    #[clap(short, long)]
    device: Option<String>,

    /// Characteristic or service uui search that will be applied on start. If one characteristic will be found matching this filter it will be selected automatically.
    #[clap(short, long)]
    characteristic: Option<String>,

    /// Customize displaying of names and services
    /// Path to file in .ini like format (with no support of [Groups]) where keys are uuids of services or characteristics and values are names to display.
    ///
    /// # Example
    ///
    /// ```
    /// 0000FFE0-0000-1000-8000-00805F9B34FB=Cpu Tempreture
    /// 4f25b5f6-01d9-4d95-86a4-81e3d2f13b8f=My Custom Service data
    /// ```
    #[clap(long, value_parser = clap::builder::ValueParser::new(parse_name_map))]
    names_map_file: Option<HashMap<uuid::Uuid, String>>,
}

#[derive(Debug)]
pub struct Ctx {
    args: Args,
    ble_manager: Manager,
    latest_scan: RwLock<Option<BleScan>>,
    active_route: RwLock<route::Route>,
    active_side_effect_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    request_scan_restart: Mutex<bool>,
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
        request_scan_restart: Mutex::new(false),
    });

    let _scanner = tokio::spawn(bluetooth::start_scan(Arc::clone(&ctx)));

    run_tui_app(ctx).unwrap();
}
