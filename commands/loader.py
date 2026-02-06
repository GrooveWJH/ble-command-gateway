"""Load command modules into dispatcher."""

from commands.builtins import BUILTIN_MODULES
from commands.registry import CommandDispatcher


def load_builtin_commands(dispatcher: CommandDispatcher) -> None:
    for module in BUILTIN_MODULES:
        module.register(dispatcher)
