"""Client application entrypoint."""

import importlib.util
import sys
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[1]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))


def _preload_rich_unicode_modules() -> None:
    """Work around Nuitka import resolution for rich unicode modules on macOS app builds."""
    try:
        import rich._unicode_data as unicode_data  # type: ignore
    except Exception:
        return

    unicode_dir = Path(unicode_data.__file__).resolve().parent
    for module_file in unicode_dir.glob("unicode*-*-*.py"):
        module_name = f"rich._unicode_data.{module_file.stem}"
        if module_name in sys.modules:
            continue
        try:
            spec = importlib.util.spec_from_file_location(module_name, module_file)
            if spec is None or spec.loader is None:
                continue
            module = importlib.util.module_from_spec(spec)
            sys.modules[module_name] = module
            spec.loader.exec_module(module)
        except Exception:
            sys.modules.pop(module_name, None)


def run() -> int:
    _preload_rich_unicode_modules()
    from client.interactive_flow import main  # noqa: E402

    return main()


if __name__ == "__main__":
    raise SystemExit(run())
