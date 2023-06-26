use crate::error::{Error, Result};
use crate::Ctx;
use btleplug::api::{Central, CharPropFlags, Manager as _, Peripheral, ScanFilter};
use futures::future::try_join_all;
use std::borrow::Cow;
use std::iter::Iterator;
use std::ops::DerefMut;
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
    pub rssi: Option<i16>,
}

#[derive(Debug, Clone)]
pub struct ConnectedCharacteristic {
    pub ble_characteristic: btleplug::api::Characteristic,
    pub standard_gatt_char_name: Option<&'static str>,
    pub standard_gatt_service_name: Option<&'static str>,
    /// Name that is coming from custom name map file (if any) from user.
    pub custom_char_name: Option<String>,
    pub custom_service_name: Option<String>,
    pub uuid: uuid::Uuid,
    pub service_uuid: uuid::Uuid,
}

impl ConnectedCharacteristic {
    pub fn char_name(&self) -> Cow<'_, str> {
        if let Some(custom_name) = &self.custom_char_name {
            return Cow::from(format!("{} ({})", custom_name, self.uuid));
        }

        if let Some(custom_name) = &self.custom_char_name {
            return Cow::from(custom_name.as_str());
        }

        if let Some(standard_name) = self.standard_gatt_char_name {
            return Cow::from(standard_name);
        }

        Cow::from(self.uuid.to_string())
    }

    pub fn service_name(&self) -> Cow<'_, str> {
        if let Some(custom_name) = &self.custom_service_name {
            return Cow::from(format!("{} ({})", custom_name, self.service_uuid));
        }

        if let Some(standard_name) = self.standard_gatt_service_name {
            return Cow::from(standard_name);
        }

        Cow::from(self.uuid.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ConnectedPeripheral {
    pub peripheral: HandledPeripheral,
    pub characteristics: Vec<ConnectedCharacteristic>,
}

impl ConnectedPeripheral {
    pub fn new(ctx: &Ctx, peripheral: HandledPeripheral) -> Self {
        // panic!("{:?}", ctx.args.names_map_file);
        let chars = peripheral.ble_peripheral.characteristics();

        Self {
            peripheral,
            characteristics: chars
                .into_iter()
                .map(|char| ConnectedCharacteristic {
                    custom_char_name: ctx
                        .args
                        .names_map_file
                        .as_ref()
                        .and_then(|names| names.get(&char.uuid).cloned()),
                    custom_service_name: ctx
                        .args
                        .names_map_file
                        .as_ref()
                        .and_then(|names| names.get(&char.uuid).cloned()),
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
                        rssi: properties.rssi,
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

        if matches!(context.request_scan_restart.lock().as_deref(), Ok(true)) {
            adapter.stop_scan().await?;
            adapter.start_scan(ScanFilter::default()).await?;

            *context.request_scan_restart.lock()?.deref_mut() = true
        }
    }
}

pub fn display_properties(props: CharPropFlags) -> String {
    let mut labels = Vec::new();

    if props.contains(CharPropFlags::BROADCAST) {
        labels.push("Broadcast");
    }
    if props.contains(CharPropFlags::READ) {
        labels.push("Read");
    }
    if props.contains(CharPropFlags::WRITE) || props.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)
    {
        labels.push("Write");
    }
    if props.contains(CharPropFlags::NOTIFY) {
        labels.push("Notify");
    }

    labels.join(", ")
}
