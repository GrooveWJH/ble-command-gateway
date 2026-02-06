# UART 指令化重构指南

## 目标

将核心 BLE 配网链路从“Wi-Fi 专用 JSON 负载”重构为“基于标准 Nordic UART UUID 的指令-动作协议”，在保持现有配网能力的前提下提升可维护性和可扩展性。

本指南基于两个已确认决策：

- `help` 返回人类可读文本。
- `shutdown` 允许在对外指令集中开放。

## 范围

- In：
  - 将核心服务端/客户端切到 UART UUID。
  - 引入统一指令 envelope 与分发器。
  - 建立完整的“指令-动作-响应”机制，并支持显式 `help`。
  - Wi-Fi 配网保留为业务指令，不再作为裸 payload 约定。
  - 保持 reset/preflight/runtime 的运维行为可用。
- Out：
  - 认证/加密体系重设计。
  - 多会话并发模型重设计。
  - UI 框架重设计。

## 目标协议

### UUID（UART / NUS）

- Service UUID：`6E400001-B5A3-F393-E0A9-E50E24DCCA9E`
- RX UUID（client write）：`6E400002-B5A3-F393-E0A9-E50E24DCCA9E`
- TX UUID（server read/notify）：`6E400003-B5A3-F393-E0A9-E50E24DCCA9E`

### 指令请求 Envelope

```json
{
  "id": "b5a7e2b2-8f1e-4f8f-9f5f-3a83b2ec16f9",
  "cmd": "provision",
  "args": {
    "ssid": "LabWiFi",
    "pwd": "secret"
  }
}
```

### 指令响应 Envelope

```json
{
  "id": "b5a7e2b2-8f1e-4f8f-9f5f-3a83b2ec16f9",
  "ok": true,
  "code": "OK",
  "text": "Connecting to LabWiFi..."
}
```

### 内置指令

- `help`
  - 在 `text` 字段返回人类可读的指令说明。
- `ping`
  - 链路健康检查。
- `status`
  - 返回当前服务状态。
- `provision`
  - 触发 Wi-Fi 配网，参数为 `ssid` 和可选 `pwd`。
- `shutdown`
  - 优雅停止服务进程。

## Good Taste 约束

- 使用命令注册表（map）分发，避免长 `if/elif` 链。
- 请求在入口处一次性校验，下游使用类型化模型并默认可信。
- BLE runtime 与业务动作解耦。
- 对未知指令/非法负载快速失败。
- 核心链路不使用隐藏 fallback。
- 函数职责单一、可测试。
- 通过 early return 控制嵌套深度。

## 分阶段计划

### Phase 1：协议与常量

- 在 `config/ble_uuid.py` 中定义 UART UUID 常量。
- 在 `protocol/envelope.py` 定义请求/响应模型。
- 增加指令解析与序列化工具。
- 如有需要，保留临时兼容开关（`legacy` vs `uart`）。

### Phase 2：服务端核心重构

- 新建 `server/command_dispatcher.py`。
- 注册处理器：`help`、`ping`、`status`、`provision`、`shutdown`。
- 重构 `app/server_main.py` 与 `ble/server_gateway.py`：
  - 仅保留 BLE 生命周期与 characteristic 绑定。
  - 将 payload 解码、命令路由、动作执行下沉到分发层。
- 保留当前配网锁行为，作为指令级 busy 保护。

### Phase 3：客户端核心重构

- 重构 `client/command_client.py`：从原始 Wi-Fi payload 改为命令 envelope。
- 增加工具方法：
  - `send_command(cmd, args, id)`
  - `wait_response(id, timeout)`
- 保持交互流程不变，但所有动作统一走命令 API。
- 新增 `help` 调用与展示路径。

### Phase 4：链路测试对齐

- 更新 `server/link_test_server.py` 与 `client/link_test_client.py`，统一使用命令 envelope。
- 用 `ping`/`status` 指令交换替换特定 hello 协议逻辑。

### Phase 5：验证与发布

- 增加 parser/dispatcher/handler 的单元与集成测试。
- 运行风格/静态检查/类型检查及现有脚本验证。
- 分阶段发布：
  - 测试环境启用 UART 模式。
  - 现场设备灰度。
  - 全量切换。
- 迁移窗口结束后移除 legacy 分支。

## 执行 Checklist

- [x] 定义 UART UUID 常量，并移除对旧 provisioning UUID 的硬依赖。
- [x] 增加命令请求/响应模型与序列化工具。
- [x] 实现严格命令解析与明确错误码。
- [x] 实现 dispatcher 注册表与 handler 接口。
- [x] 实现 `help` handler，并返回人类可读文本。
- [x] 实现 `shutdown` handler，并打通优雅停机。
- [x] 将 `provision` 流程迁入命令 handler 并保留状态通知。
- [x] 将客户端读写链路重构为命令 envelope。
- [x] 更新交互客户端动作，改为命令 API 调用。
- [x] 将链路测试 client/server 对齐到新命令协议。
- [x] 更新文档（`README.md`、`README_EN.md`）并加入 UART 协议示例。
- [x] 补充 unknown command、invalid JSON、missing fields、busy lock、shutdown 测试。

## 指令管理系统 Checklist

- [x] 统一命令命名规范（新增 `sys.whoami`、`net.ifconfig`）。
- [x] 建立声明式命令注册（`CommandSpec` + `register()`）。
- [x] 建立统一执行管线（参数校验、超时、异常映射）。
- [x] `help` 展示命令摘要与运维字段（permission/risk/timeout）。
- [x] 建立系统命令白名单执行层（禁止任意 shell 透传）。
- [x] 提供新增命令文档模板（`docs/COMMAND_AUTHORING.md`）。

## Good Taste 清理 Checklist

- [x] 统一命令 ID 来源：服务端/客户端/测试全部使用同一个常量模块。
- [x] 删除协议层死代码：移除未使用的 `SUPPORTED_COMMANDS` 与过时 `help_text()`。
- [x] 修复 `ifconfig` 路径解析逻辑，避免返回不可执行路径。
- [x] 让 `scripts/bless_uart.py` 退出核心路径（迁移到 `tools/legacy` 或标记弃用）。
- [x] 补充对应测试并完成一次全量编译+单测验证。

## Good Taste 第二批 Checklist

- [x] 配网日志完全脱敏（`pwd` 不落日志）。
- [x] 从 `wifi_ble_service.py` 拆出 `WifiProvisioningService`。
- [x] 从 `wifi_ble_service.py` 拆出 `ResponsePublisher`，收敛响应状态与发送逻辑。
- [x] legacy 脚本默认拒绝执行，必须显式 `--run-legacy` 才可运行。

## 目录迁移 Checklist

- [x] 新增 `app/` 入口层（`app/server_main.py`、`app/client_main.py`）。
- [x] 新增 `ble/` 分层（`ble/server_gateway.py`、`ble/runtime.py`、`ble/response_publisher.py`）。
- [x] 新增 `ble/scan_transport.py` 作为客户端扫描/连接基础传输层。
- [x] 新增 `protocol/` 分层（`protocol/envelope.py`、`protocol/command_ids.py`、`protocol/codes.py`）。
- [x] 新增 `commands/` 分层（`commands/registry.py`、`commands/loader.py`、`commands/schemas.py`、`commands/builtins/`）。
- [x] 新增 `services/` 分层（`services/wifi_provisioning_service.py`、`services/system_exec_service.py`）。
- [x] 新增 `config/` 分层（`config/ble_uuid.py`、`config/defaults.py`），并移除 `config.py`。
- [x] 客户端入口收敛为 `client/interactive_flow.py` + `client/command_client.py` + `client/models.py`。
- [x] 运维/legacy 脚本迁移到 `tools/reset/server_reset.py` 与 `tools/legacy/bless_uart_demo.py`。
- [x] 测试目录按职责分级到 `tests/unit`、`tests/integration`、`tests/e2e`。
- [x] 旧兼容入口已删除，统一使用 `app/server_main.py` 与 `app/client_main.py`。
- [x] 迁移过程中保持旧命令可运行并通过测试。

## 验收 Checklist

- [ ] 客户端可执行 `help` 并收到可读的指令说明。
- [ ] 客户端可执行 `provision` 并收到预期状态流转。
- [ ] 客户端可执行 `status` 并获得当前服务状态。
- [ ] 客户端可执行 `shutdown` 且服务端优雅退出。
- [ ] 未知指令返回确定性的错误响应。
- [ ] 非法 payload 返回确定性的解析/校验错误。
- [ ] 链路测试与配网主流程使用同一套命令协议。
- [ ] BLE callback 体内不再保留直接业务逻辑。
- [ ] 核心命令链路有测试覆盖，并通过本地/CI 检查。

## 风险与缓解

- 风险：`help` 文本较长导致 BLE 包分片问题。
  - 缓解：保持 `help` 输出简洁；必要时加入分片/分页策略。
- 风险：现网仍有旧客户端使用 legacy payload。
  - 缓解：保留临时协议模式开关，并公布迁移截止时间。
- 风险：开放 `shutdown` 可能被附近设备滥用。
  - 缓解：保留命令审计日志，后续可增加认证门禁。

## 待确认问题

- `help` 是否需要支持多层级，如 `help` 与 `help <cmd>`？
- `id` 是否必须由客户端提供，还是允许服务端兜底生成？
- `shutdown` 应该先 ACK 再停止，还是停止后再回最终结果？
