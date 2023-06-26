use super::Ctx;
use crate::{
    bluetooth::{self, ConnectedCharacteristic, ConnectedPeripheral},
    error,
};
use btleplug::api::Peripheral;
use std::{
    ops::{Deref, DerefMut},
    sync::{atomic::AtomicU16, Arc, RwLock},
    time::Duration,
};
use tokio::time::{self, timeout};

use crate::bluetooth::HandledPeripheral;

#[derive(Debug, Clone)]
pub struct CharacteristicValue {
    pub time: chrono::DateTime<chrono::Local>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum Route {
    PeripheralList,
    PeripheralConnectedView(ConnectedPeripheral),
    PeripheralWaitingView {
        peripheral: HandledPeripheral,
        retry: Arc<AtomicU16>,
    },
    CharacteristicView {
        peripheral: ConnectedPeripheral,
        characteristic: ConnectedCharacteristic,
        value: Arc<RwLock<Option<CharacteristicValue>>>,
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
                    value,
                },
            ) => loop {
                let ble_peripheral = &peripheral.peripheral.ble_peripheral;
                if let Ok(data) = ble_peripheral
                    .read(&characteristic.ble_characteristic)
                    .await
                {
                    value.write().unwrap().replace(CharacteristicValue {
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
