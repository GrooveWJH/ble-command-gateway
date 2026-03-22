# 客户端库 API (Rust)

为了方便跨平台操作，所有的蓝牙核心逻辑都被提取到了 `client` crate 当中。它基于 `btleplug` 构建，提供了异步的 `tokio` 接口用于连接设备、扫描附近信标以及直接发送/接收底层指令。

它被作为底层引擎驱动着 `gui` 桌面客户端和命令行终端。

## 代码调用示例

```rust
use client::BleClient;
use protocol::commands::CMD_PING;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BleClient::new().await?;
    
    // 1. 扫描具有特定名称前缀的设备广播 (例如 Yundrone_UAV)
    let device = client.scan_for_device("Yundrone_UAV", 10).await?;
    
    // 2. 建立底层连接
    // client.connect_to_device(&device).await?;
    
    // 3. 发现服务并监听回调
    // let (write_ch, read_ch) = client.discover_characteristics(&device).await?;
    // client.subscribe_notifications(&device, &read_ch).await?;
    
    // 4. 发送 JSON 指令载荷
    // client.send_payload(&device, &write_ch, b"{\"cmd\":\"status\"}").await?;
    
    Ok(())
}
```

由于全部切换到了 Rust 架构，客户端天然使用 `async` 异步流程。如果您不得不在同步代码块中调用它，可以使用 `tokio::runtime::Runtime::new().unwrap().block_on(...)` 进行阻塞执行。
