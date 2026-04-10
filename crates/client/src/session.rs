use anyhow::{anyhow, Result};
use btleplug::api::{CharPropFlags, Characteristic, Peripheral as _, ValueNotification, WriteType};
use btleplug::platform::Peripheral;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::time::Duration;
use tracing::info;

pub struct BleSession {
    device_name: String,
    device_rssi: Option<i16>,
    device: Peripheral,
    write_char: Characteristic,
    read_char: Characteristic,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    response_decoder: crate::response::ResponseDecoder,
}

impl BleSession {
    pub(crate) async fn connect(
        device_name: String,
        device_rssi: Option<i16>,
        device: Peripheral,
    ) -> Result<Self> {
        info!(
            device_name = %device_name,
            rssi = ?device_rssi,
            "ble.session.connecting"
        );
        device.connect().await?;
        device.discover_services().await?;
        let chars = device.characteristics();

        let write_uuid = uuid::Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?;
        let read_uuid = uuid::Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?;

        let mut write_char = None;
        let mut read_char = None;

        for c in chars {
            if c.uuid == write_uuid && c.properties.contains(CharPropFlags::WRITE) {
                write_char = Some(c.clone());
            } else if c.uuid == read_uuid
                && c.properties
                    .intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE)
            {
                read_char = Some(c.clone());
            }
        }

        let write_char = write_char.ok_or_else(|| anyhow!("Write characteristic not found"))?;
        let read_char = read_char.ok_or_else(|| anyhow!("Read/Notify characteristic not found"))?;
        device.subscribe(&read_char).await?;
        let notifications = device.notifications().await?;

        info!(
            device_name = %device_name,
            rssi = ?device_rssi,
            "ble.session.ready"
        );

        Ok(Self {
            device_name,
            device_rssi,
            device,
            write_char,
            read_char,
            notifications,
            response_decoder: crate::response::ResponseDecoder::new(),
        })
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn device_rssi(&self) -> Option<i16> {
        self.device_rssi
    }

    pub async fn send_payload(&self, payload: &[u8]) -> Result<()> {
        self.device
            .write(&self.write_char, payload, WriteType::WithoutResponse)
            .await?;
        Ok(())
    }

    pub async fn send_request(&self, request: &crate::PreparedRequest) -> Result<()> {
        info!(
            device_name = %self.device_name,
            rssi = ?self.device_rssi,
            cmd = %request.request.payload.command_name(),
            request_id = %request.request.id,
            payload_bytes = request.bytes.len(),
            "ble.request.sent"
        );
        self.send_payload(&request.bytes).await
    }

    pub async fn next_response(&mut self, timeout_secs: u64) -> Result<protocol::CommandResponse> {
        tokio::time::timeout(Duration::from_secs(timeout_secs), async {
            while let Some(notification) = self.notifications.next().await {
                if notification.uuid != self.read_char.uuid {
                    continue;
                }

                match self.response_decoder.decode(&notification.value) {
                    Ok(Some(response)) => {
                        info!(
                            device_name = %self.device_name,
                            rssi = ?self.device_rssi,
                            response_id = %response.id,
                            ok = response.ok,
                            code = %response.code,
                            "ble.response.received"
                        );
                        return Ok(response);
                    }
                    Ok(None) => continue,
                    Err(err) => return Err(anyhow!(err.to_string())),
                }
            }

            Err(anyhow!(
                "Notification stream closed before a response was received"
            ))
        })
        .await
        .map_err(|_| anyhow!("Timed out waiting for BLE response after {}s", timeout_secs))?
    }
}
