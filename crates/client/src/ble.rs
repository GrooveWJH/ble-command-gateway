use std::time::Duration;
use anyhow::{Result, anyhow};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, CharPropFlags, Characteristic, WriteType};
use btleplug::platform::{Manager, Adapter, Peripheral};
use tracing::{info, debug};

pub struct BleClient {
    adapter: Adapter,
}

impl BleClient {
    pub async fn new() -> Result<Self> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        
        let adapter = adapters.into_iter().nth(0)
            .ok_or_else(|| anyhow!("No Bluetooth adapters found"))?;
            
        Ok(Self { adapter })
    }

    pub async fn scan_for_device(&self, prefix: &str, timeout_secs: u64) -> Result<btleplug::platform::Peripheral> {
        info!("Starting scan for device with prefix '{}'...", prefix);
        self.adapter.start_scan(ScanFilter::default()).await?;
        
        tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
        
        let peripherals = self.adapter.peripherals().await?;
        let mut target_peripheral = None;
        
        for peripheral in peripherals {
            if let Some(properties) = peripheral.properties().await? {
                if let Some(name) = properties.local_name {
                    debug!("Found device: {}", name);
                    if name.starts_with(prefix) {
                        target_peripheral = Some(peripheral);
                        break;
                    }
                }
            }
        }
        
        self.adapter.stop_scan().await?;
        
        target_peripheral.ok_or_else(|| anyhow!("Device '{}' not found after {}s scan", prefix, timeout_secs))
    }

    pub async fn connect_to_device(&self, device: &Peripheral) -> Result<()> {
        info!("Connecting to device...");
        device.connect().await?;
        Ok(())
    }

    pub async fn discover_characteristics(&self, device: &Peripheral) -> Result<(Characteristic, Characteristic)> {
        device.discover_services().await?;
        let chars = device.characteristics();
        
        // Protocol specified characteristics
        let write_uuid = uuid::Uuid::parse_str("6E400002-B5A3-F393-E0A9-E50E24DCCA9E")?;
        let read_uuid = uuid::Uuid::parse_str("6E400003-B5A3-F393-E0A9-E50E24DCCA9E")?;
        
        let mut write_char = None;
        let mut read_char = None;

        for c in chars {
            if c.uuid == write_uuid && c.properties.contains(CharPropFlags::WRITE) {
                write_char = Some(c.clone());
            } else if c.uuid == read_uuid && c.properties.intersects(CharPropFlags::NOTIFY | CharPropFlags::INDICATE) {
                read_char = Some(c.clone());
            }
        }

        let w = write_char.ok_or_else(|| anyhow!("Write characteristic not found"))?;
        let r = read_char.ok_or_else(|| anyhow!("Read/Notify characteristic not found"))?;
        
        Ok((w, r))
    }

    pub async fn subscribe_notifications(&self, device: &Peripheral, read_char: &Characteristic) -> Result<()> {
        info!("Subscribing to notifications...");
        device.subscribe(read_char).await?;
        Ok(())
    }

    pub async fn send_payload(&self, device: &Peripheral, write_char: &Characteristic, payload: &[u8]) -> Result<()> {
        debug!("Sending payload of {} bytes", payload.len());
        // For actual production, MTU fragmentation should be applied before this call.
        device.write(write_char, payload, WriteType::WithoutResponse).await?;
        Ok(())
    }
}
