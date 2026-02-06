# 指令扩展指南

目标：新增一个指令时，不改核心分发器逻辑，仅新增模块并注册。

## 三步新增指令

1. 在 `protocol/command_ids.py` 新增命令常量。  
2. 在 `commands/builtins/` 下新增 `<name>_cmd.py`。  
3. 在文件中定义 `SPEC`（`CommandSpec`）和 `register(dispatcher)`。  
4. 将模块加入 `commands/builtins/__init__.py` 的 `BUILTIN_MODULES`。

## 模块模板

```python
from protocol.command_ids import CMD_EXAMPLE_ECHO
from commands.schemas import CommandSpec
from protocol.envelope import CommandRequest, CommandResponse, response_ok
from commands.registry import CommandDispatcher, DispatchContext

SPEC = CommandSpec(
    name=CMD_EXAMPLE_ECHO,
    summary="Echo input text",
    usage="example.echo {text: str}",
    permission="user",
    risk="low",
    timeout_sec=2.0,
)

def register(dispatcher: CommandDispatcher) -> None:
    async def _handler(_context: DispatchContext, request: CommandRequest) -> CommandResponse:
        text = str(request.args.get("text", ""))
        return response_ok(request.request_id, text)

    dispatcher.register(SPEC, _handler)
```

## 约束

- 指令名必须唯一，重复注册会报错。
- 命令 ID 必须集中定义在 `protocol/command_ids.py`，不要在模块内写裸字符串。
- 参数校验由 `CommandSpec.args` 自动完成，避免在 handler 里重复做样板校验。
- `help` 文本自动来自注册表，会展示 usage、permission、risk、timeout。
- 系统命令必须走 `run_system_command` 白名单，不允许任意 shell 透传。
