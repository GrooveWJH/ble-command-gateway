# 客户端库 API (Rust)

`client` crate 的推荐入口是 `BleClient + BleSession`。上层不再需要自己拼“扫描 / 连接 / 找特征 / 订阅 / 编解码”这条链路。

## 推荐调用流程

1. 用稳定前缀扫描候选设备，例如 `Yundrone_UAV`
2. 让用户从候选列表中选择具体广播实例
3. 用 `connect_session(...)` 建立 `BleSession`
4. 用 `prepare_request(...)` 构造带 request ID 的 typed 请求
5. 通过 `session.send_request(...)` 发送请求
6. 通过 `session.next_response(...)` 等待 `CommandResponse`
7. 用 `response.decode_data::<T>()` 解出 typed response data

## 主要公开接口

- `BleClient::scan_candidates(prefix, timeout_secs)`
  返回命中前缀的候选设备列表
- `BleClient::scan_candidates_with_progress(prefix, timeout_secs, on_event)`
  扫描期间通过 `ScanProgressEvent` 实时上报已发现的命名设备，适合 GUI Raw Logs
- `BleClient::connect_session(device)`
  建立连接并返回 `BleSession`
- `prepare_request(payload)`
  根据 `CommandPayload` 生成带 request ID 的请求
- `BleSession::send_request(...)`
  发送结构化请求
- `BleSession::next_response(timeout_secs)`
  读取并重组通知响应
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
    let mut candidates = client.scan_candidates("Yundrone_UAV", 10).await?;
    let device = candidates.remove(0);
    let mut session = client.connect_session(device).await?;

    let request = prepare_request(CommandPayload::Status)?;
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

服务端默认广播名是动态实例名，例如 `Yundrone_UAV-15-19-A7F2`。推荐始终按稳定前缀扫描，再由 CLI / GUI 让用户选择最终设备，而不是默认连第一个命中项。

如果你只需要发送底层字节，仍可使用 `session.send_payload(...)`；但正式命令链路推荐统一走 `prepare_request(...) + send_request(...)`，这样 request ID、日志字段和协议兼容性会保持一致。
