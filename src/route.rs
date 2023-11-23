use super::Ctx;
use crate::{
    bluetooth::{self, ConnectedCharacteristic, ConnectedPeripheral},
    error,
};
use btleplug::api::Peripheral;
use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicIsize, AtomicU16},
        Arc, RwLock,
    },
    time::Duration,
};
use tokio::time::{self, timeout};

use crate::bluetooth::HandledPeripheral;

#[derive(Debug, Clone)]
pub struct CharacteristicValue {
    pub time: chrono::DateTime<chrono::Local>,
    pub data: Vec<u8>,
}

/// Atomic implementation for optional index
#[derive(Debug)]
pub struct AtomicOptionalIndex(AtomicIsize);

impl Default for AtomicOptionalIndex {
    fn default() -> Self {
        Self(AtomicIsize::new(-1))
    }
}

impl AtomicOptionalIndex {
    pub fn read(&self) -> Option<usize> {
        let value = self.0.load(std::sync::atomic::Ordering::SeqCst);

        if value < 0 {
            None
        } else {
            Some(value as usize)
        }
    }

    pub fn write(&self, value: usize) {
        let new_value: isize = if let Ok(new_value) = value.try_into() {
            new_value
        } else {
            tracing::error!(
                "Failed to convert atomic optional index. Falling back to the isize max"
            );

            isize::MAX
        };

        self.0.store(new_value, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn annulate(&self) {
        self.0.store(-1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub enum Route {
    PeripheralList,
    PeripheralConnectedView(ConnectedPeripheral),
    PeripheralWaitingView {
        peripheral: HandledPeripheral,
        retry: Arc<AtomicU16>,
    },
    // todo pull out into separate struct with default impl
    CharacteristicView {
        peripheral: ConnectedPeripheral,
        characteristic: ConnectedCharacteristic,
        historical_view_index: Arc<AtomicOptionalIndex>,
        history: Arc<RwLock<Vec<CharacteristicValue>>>,
    },
}

#[allow(clippy::single_match)]
impl Route {
    pub(crate) async fn spawn_navigation_side_effect(
        self,
        previous: &Route,
        ctx: &Ctx,
    ) -> error::Result<()> {
        match (previous, self) {
            (Route::PeripheralList, Route::PeripheralWaitingView { peripheral, retry }) => {
                while peripheral
                    .ble_peripheral
                    .is_connected()
                    .await
                    .map(|c| !c)
                    .unwrap_or(true)
                {
                    if let Err(e) =
                        timeout(Duration::from_secs(2), peripheral.ble_peripheral.connect()).await
                    {
                        tracing::error!(?e, "Failed to connect to peripheral.");
                    }

                    retry.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }

                peripheral.ble_peripheral.discover_services().await?;
                let mut active_route = ctx.active_route.write().unwrap();
                (*active_route) =
                    Route::PeripheralConnectedView(ConnectedPeripheral::new(ctx, peripheral))
            }
            (
                Route::PeripheralConnectedView(ConnectedPeripheral { peripheral, .. }),
                Route::PeripheralList,
            ) => {
                bluetooth::disconnect_with_timeout(&peripheral.ble_peripheral).await;
            }
            (
                _,
                Route::CharacteristicView {
                    peripheral,
                    characteristic,
                    history,
                    ..
                },
            ) => loop {
                let ble_peripheral = &peripheral.peripheral.ble_peripheral;
                if let Ok(data) = ble_peripheral
                    .read(&characteristic.ble_characteristic)
                    .await
                {
                    history.write().unwrap().push(CharacteristicValue {
                        time: chrono::Local::now(),
                        data,
                    });

                    time::sleep(Duration::from_millis(ctx.args.scan_interval)).await;
                }
            },

            _ => (),
        }

        Ok(())
    }

    pub fn navigate(self, ctx: &Arc<Ctx>) {
        let active_route = ctx.active_route.write();
        let mut active_route = active_route
            .map_err(|e| {
                tracing::error!(?e, "Failed to acquire active route lock.");
            })
            .unwrap();

        tracing::debug!("Navigating from {:?} to {:?}", active_route, self);

        let old_route = std::mem::replace(active_route.deref_mut(), self.clone());
        drop(active_route);

        let ctx_clone = Arc::clone(ctx);
        let active_handle = tokio::spawn(async move {
            if let Err(e) = self
                .spawn_navigation_side_effect(&old_route, &ctx_clone)
                .await
            {
                tracing::error!("Failed to perform navigation side effect: {:?}", e);

                if let Ok(global_error) = ctx_clone.global_error.lock().as_deref_mut() {
                    *global_error = Some(e);
                }
            }
        });

        let mut effect_handle = ctx.active_side_effect_handle.lock().unwrap();
        if let Some(handle) = effect_handle.deref() {
            handle.abort();
        }

        effect_handle.replace(active_handle);
    }
}
