use crate::error::{Error, Result};
use crate::Ctx;
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use futures::future::try_join_all;
use std::iter::Iterator;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{self, sleep, timeout};

pub mod ble_default_services;

const TIMEOUT: Duration = Duration::from_secs(10);

pub async fn disconnect_with_timeout(peripheral: &btleplug::platform::Peripheral) {
    match timeout(TIMEOUT, peripheral.is_connected()).await {
        Ok(Ok(false)) => {
            return;
        }
        e => {
            tracing::error!(
                "It looks like peripheral connection died on its own: {:?}",
                e
            );
        }
    }

    loop {
        if let Err(e) = timeout(TIMEOUT, peripheral.disconnect()).await {
            tracing::error!("Error while disconnecting: {e:?}. Will try again in 5 seconds");
        } else {
            break;
        }

        sleep(Duration::from_secs(5)).await;
    }
}

#[derive(Debug, Clone)]
pub struct HandledPeripheral<TPer: Peripheral = btleplug::platform::Peripheral> {
    pub name_unset: bool,
    pub ble_peripheral: TPer,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ConnectedCharacteristic {
    pub ble_characteristic: btleplug::api::Characteristic,
    pub standard_gatt_char_name: Option<&'static str>,
    pub standard_gatt_service_name: Option<&'static str>,
    pub uuid: uuid::Uuid,
    pub service_uuid: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct ConnectedPeripheral {
    pub peripheral: HandledPeripheral,
    pub characteristics: Vec<ConnectedCharacteristic>,
}

impl ConnectedPeripheral {
    pub fn new(peripheral: HandledPeripheral) -> Self {
        let chars = peripheral.ble_peripheral.characteristics();

        Self {
            peripheral,
            characteristics: chars
                .into_iter()
                .map(|char| ConnectedCharacteristic {
                    standard_gatt_char_name: ble_default_services::SPECIAL_CHARACTERISTICS_NAMES
                        .get(&char.uuid)
                        .copied(),
                    standard_gatt_service_name: ble_default_services::SPECIAL_SERVICES_NAMES
                        .get(&char.service_uuid)
                        .copied(),
                    uuid: char.uuid,
                    service_uuid: char.service_uuid,
                    ble_characteristic: char,
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct BleScan {
    pub peripherals: Vec<HandledPeripheral>,
    pub sync_time: chrono::DateTime<chrono::Local>,
}

pub async fn start_scan(context: Arc<Ctx>) -> Result<()> {
    let adapter_list = context.ble_manager.adapters().await?;

    if adapter_list.is_empty() {
        return Err(
            Error::client("No BLE adapters found. Looks like your device doesn't have a Bluetooth Low Energy adapter or drivers are not installed.")
        );
    }

    let adapter = &adapter_list[context.args.adapter_index];
    adapter.start_scan(ScanFilter::default()).await?;

    loop {
        let peripherals = adapter.peripherals().await?;
        let properties_futures = peripherals
            .iter()
            .map(Peripheral::properties)
            .collect::<Vec<_>>();

        let peripherals = try_join_all(properties_futures)
            .await?
            .into_iter()
            .zip(peripherals.into_iter())
            .flat_map(|(properties, peripheral)| {
                properties.map(|properties| {
                    let name_unset = properties.local_name.is_none();
                    let name = properties
                        .local_name
                        .unwrap_or_else(|| "Unknown device".to_string());

                    HandledPeripheral {
                        ble_peripheral: peripheral,
                        name,
                        name_unset,
                    }
                })
            })
            .collect::<Vec<_>>();

        context.latest_scan.write()?.replace(BleScan {
            peripherals,
            sync_time: chrono::Local::now(),
        });

        time::sleep(Duration::from_millis(context.args.scan_interval)).await;
    }
}
