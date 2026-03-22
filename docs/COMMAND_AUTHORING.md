# 自定义指令开发指南

## 理解新的架构层级

在 Rust 版本的重构中，新增或修改一条蓝牙指令，需要跨越以下几个模块层：

1. **协议定义层**: `crates/protocol/src/lib.rs`
2. **服务端执行层**: `crates/server/src/services.rs`
3. **客户端发包层**: `crates/client/` 或 `crates/gui/`

### 1. 注册新的指令 ID

打开 `crates/protocol/src/lib.rs` 并找到 `pub mod commands` 模块。在这里添加表示您新指令的字符串常量：

```rust
pub mod commands {
    pub const CMD_HELP: &str = "help";
    pub const CMD_MY_NEW_THING: &str = "my.new_thing"; // <-- 新增
}
```

### 2. 在服务端实现业务处理逻辑

打开 `crates/server/src/services.rs`，将其添加到全局的异步指令路由分发器 `run_named_command` 中：

```rust
pub async fn run_named_command(name: &str, ifname: Option<&str>, timeout_sec: f64) -> SystemExecResult {
    match name {
        protocol::commands::CMD_MY_NEW_THING => handle_new_thing().await,
        // ...
// ...
```

随后在文件下方编写对应的 async 异步函数。如果您的业务需要调用底层 Linux 的 shell （比如驱动脚本），可以直接基于 `tokio::process::Command` 派生子进程去抓取结果并返回 `SystemExecResult`。

### 3. 在客户端或 GUI 发起请求

如果您的新命令需要携带额外参数（比如附带 `ssid` 和 `password`），请在包装发往 `client.send_payload` 时使用 `serde_json::json!` 宏进行直观的构造：

```rust
let req = CommandRequest::new(
    "req-1",
    protocol::commands::CMD_MY_NEW_THING,
    Some(json!({"param1": "foo"}).as_object().unwrap().clone())
);
```

完成代码逻辑后，通过 `cargo build` 重新编译您的 Workspace 即可生效！
