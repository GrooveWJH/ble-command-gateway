# 客户端库 API (Rust)

`client` crate 的推荐入口是 `BleClient + BleSession`。上层不再需要自己拼“扫描 / 连接 / 找特征 / 订阅 / 编解码”这条链路。

## 推荐调用流程

1. 用稳定前缀扫描候选设备，例如 `yundrone`
2. 让用户从候选列表中选择具体广播实例
3. 用 `connect_session(...)` 建立 `BleSession`
4. 用 `prepare_request(...)` 构造带 request ID 的 typed 请求
5. 通过 `session.send_request(...)` 发送请求
6. 对快速命令可通过 `session.next_response(...)` 等待单个 `CommandResponse`
7. 对 `wifi.scan` / `wifi.provision` / `wifi.profiles.delete` 这类耗时命令，推荐用 `session.run_request_until_final(...)` 消费事件直到 `final=true`
8. 用最终响应的 `response.decode_data::<T>()` 解出 typed response data

## 主要公开接口

- `BleClient::scan_candidates(prefix, timeout_secs)`
  返回名称匹配稳定前缀的候选设备列表，连接后再通过 GATT 服务发现完成最终验证
- `BleClient::scan_candidates_with_progress(prefix, timeout_secs, on_event)`
  扫描期间通过 `ScanProgressEvent` 实时上报已发现的命名设备，适合 GUI Raw Logs
- `BleClient::connect_session(device)`
  建立连接并返回 `BleSession`
- `prepare_request(payload)`
  根据 `CommandPayload` 生成带 request ID 的请求
- `BleSession::send_request(...)`
  发送结构化请求
- `BleSession::next_response(timeout_secs)`
  读取并重组下一条通知响应事件
- `BleSession::next_event(timeout_secs)`
  与 `next_response` 等价，语义上表示读取 V2 事件
- `BleSession::run_request_until_final(request, timeout_secs, on_event)`
  发送请求，并在回调里交付 `accepted/progress/result` 事件，直到最终 `final=true`
- `CommandResponse::decode_data::<T>()`
  将 `data` 解码为 `protocol::responses::*` 中的 typed 结构

## 示例

```rust
use client::{prepare_request, BleClient};
use protocol::requests::CommandPayload;
use protocol::responses::StatusResponseData;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BleClient::new().await?;
    let mut candidates = client.scan_candidates("yundrone", 10).await?;
    let device = candidates.remove(0);
    let mut session = client.connect_session(device).await?;

    let request = prepare_request(CommandPayload::SystemStatus)?;
    session.send_request(&request).await?;
    let response = session.next_response(10).await?;
    let status: StatusResponseData = response.decode_data()?;

    println!(
        "device={} hostname={} user={}",
        session.device_name(),
        status.hostname,
        status.user
    );
    Ok(())
}
```

耗时命令示例：

```rust
use client::{prepare_request, BleSession};
use protocol::requests::CommandPayload;
use protocol::responses::WifiScanResponseData;

async fn scan_wifi(session: &mut BleSession) -> anyhow::Result<WifiScanResponseData> {
    let request = prepare_request(CommandPayload::WifiScan { ifname: None })?;
    let response = session
        .run_request_until_final(&request, 30, |event| {
            if !event.final_flag {
                eprintln!("{} #{}: {}", event.phase.as_str(), event.seq, event.text);
            }
        })
        .await?;
    Ok(response.decode_data()?)
}
```

服务端默认只广播一个 BLE local name（如 `yundrone-ytcwln`）。推荐始终按稳定前缀扫描，再由 CLI / GUI 让用户选择最终设备，而不是默认连第一个命中项。

如果你只需要发送底层字节，仍可使用 `session.send_payload(...)`；但正式命令链路推荐统一走 `prepare_request(...) + send_request(...)`，这样 request ID、日志字段和协议兼容性会保持一致。
