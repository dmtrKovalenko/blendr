use btleplug::api::Peripheral;

use super::Ctx;
use crate::{
    bluetooth::{self, ConnectedCharacteristic, ConnectedPeripheral},
    error,
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::bluetooth::HandledPeripheral;

#[derive(Debug, Clone)]
pub enum Route {
    PeripheralList,
    PeripheralWaitingView { peripheral: HandledPeripheral },
    PeripheralConnectedView(ConnectedPeripheral),
    CharacteristicView(ConnectedCharacteristic),
}

#[allow(clippy::single_match)]
impl Route {
    pub(crate) async fn navigation_side_effect(
        self,
        previous: &Route,
        ctx: Arc<Ctx>,
    ) -> error::Result<()> {
        match (previous, self) {
            (Route::PeripheralList, Route::PeripheralWaitingView { peripheral }) => {
                while peripheral
                    .ble_peripheral
                    .is_connected()
                    .await
                    .map(|c| !c)
                    .unwrap_or(true)
                {
                    peripheral.ble_peripheral.connect().await?;
                }

                let mut active_route = ctx.active_route.write().unwrap();

                (*active_route) =
                    Route::PeripheralConnectedView(ConnectedPeripheral::new(peripheral))
            }
            (
                Route::PeripheralConnectedView(ConnectedPeripheral { peripheral, .. }),
                Route::PeripheralList,
            ) => {
                bluetooth::disconnect_with_timeout(&peripheral.ble_peripheral).await;
            }
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
            if let Err(e) = self.navigation_side_effect(&old_route, ctx_clone).await {
                tracing::error!("Failed to perform navigation side effect: {:?}", e);
            }
        });

        let mut effect_handle = ctx.active_side_effect_handle.lock().unwrap();
        if let Some(handle) = effect_handle.deref() {
            handle.abort();
        }

        effect_handle.replace(active_handle);
    }
}
