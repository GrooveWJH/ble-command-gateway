# 自定义指令开发指南

新增或修改一条 BLE 指令时，推荐沿着“协议定义 -> 服务端处理 -> 客户端调用 -> 回归测试”这条固定路径推进，不要回退到字符串分发或手写 JSON map。

## 1. 在协议层新增 typed payload

先修改 `crates/protocol/src/requests.rs`，为新命令增加 `CommandPayload` 变体。

如果命令需要线格式字符串常量，再同步更新 `crates/protocol/src/lib.rs` 中的 `commands` 模块。这里的字符串只服务于 wire format，不应重新变成 server 内部的主调度入口。

示例：

```rust
pub enum CommandPayload {
    Help,
    Ping,
    MyNewThing { param1: String },
}
```

## 2. 如有返回数据，为响应定义 typed data

在 `crates/protocol/src/responses.rs` 中新增对应的响应结构，保证 CLI / GUI / server 都通过统一类型交互。

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MyNewThingResponseData {
    pub result: String,
}
```

## 3. 在 server 中接入 typed handler

修改 `crates/server/src/services/mod.rs`，把新命令接到 `run_payload_command(...)` 的 `match` 分支里。

```rust
match payload {
    protocol::requests::CommandPayload::MyNewThing { param1 } => {
        my_module::run_my_new_thing(param1, timeout_sec).await
    }
    // ...
}
```

随后在 `system_commands.rs`、`network.rs` 或新拆出的服务模块中实现 handler。handler 应直接接 typed 参数，而不是再从 `Map<String, Value>` 里二次解析协议字段。

## 4. 在 client / GUI 侧统一走 prepare_request

上层请求统一通过 `prepare_request(...)` 生成，再由 `BleSession` 发送：

```rust
let request = prepare_request(protocol::requests::CommandPayload::MyNewThing {
    param1: "foo".to_string(),
})?;
session.send_request(&request).await?;
```

如果需要读取结构化返回值，直接使用：

```rust
let response = session.next_response(10).await?;
let data: protocol::responses::MyNewThingResponseData = response.decode_data()?;
```

## 5. 补齐回归测试

至少补这三类测试：

- `cargo test -p protocol`
  验证请求编解码和响应数据 round-trip
- `cargo test -p server`
  验证 typed dispatch 与 handler 行为
- `cargo test -p client`
  如果 client/GUI 新增了解码或展示辅助逻辑，补对应纯函数测试

如果命令会暴露在 GUI 中，再补 `cargo test -p gui` 相关状态流转测试。
