from __future__ import annotations

from typing import Any, cast

from client.gui.state import GuiState

try:
    import FreeSimpleGUI as sg
except Exception as exc:  # noqa: BLE001
    if isinstance(exc, ModuleNotFoundError) and exc.name in {"_tkinter", "tkinter"}:
        raise RuntimeError(
            "FreeSimpleGUI 已安装，但当前 Python 缺少 tkinter/_tkinter。"
            "请先安装 Tk 运行时（例如 macOS Homebrew: brew install python-tk@3.11），"
            "然后使用 uv run python app/client_gui_main.py 启动。"
        ) from exc
    raise RuntimeError("FreeSimpleGUI is required for GUI client. Install with: uv sync --group gui") from exc

KEY_TARGET_NAME = "-TARGET-NAME-"
KEY_SCAN_TIMEOUT = "-SCAN-TIMEOUT-"
KEY_WAIT_TIMEOUT = "-WAIT-TIMEOUT-"
KEY_VERBOSE = "-VERBOSE-"

KEY_SCAN = "-SCAN-"
KEY_SCAN_SUMMARY = "-SCAN-SUMMARY-"
KEY_DEVICE_TABLE = "-DEVICE-TABLE-"
KEY_CONNECT = "-CONNECT-"
KEY_DISCONNECT = "-DISCONNECT-"
KEY_SESSION_STATE = "-SESSION-STATE-"
KEY_HEALTH_DOT = "-HEALTH-DOT-"
KEY_HEALTH_TEXT = "-HEALTH-TEXT-"

KEY_SSID = "-SSID-"
KEY_PASSWORD = "-PASSWORD-"
KEY_SAVE_WIFI = "-SAVE-WIFI-"
KEY_PROVISION = "-PROVISION-"

KEY_DIAG_STATUS = "-DIAG-STATUS-"
KEY_DIAG_WIFI_SCAN = "-DIAG-WIFI-SCAN-"
KEY_DIAG_PING = "-DIAG-PING-"
KEY_DIAG_HELP = "-DIAG-HELP-"

KEY_RESULT_TABS = "-RESULT-TABS-"
KEY_TAB_OVERVIEW = "-TAB-OVERVIEW-"
KEY_TAB_STATUS = "-TAB-STATUS-"
KEY_TAB_WIFI = "-TAB-WIFI-"
KEY_TAB_RAW = "-TAB-RAW-"
KEY_RESULT_OVERVIEW = "-RESULT-OVERVIEW-"
KEY_STATUS_SUMMARY = "-STATUS-SUMMARY-"
KEY_STATUS_TABLE = "-STATUS-TABLE-"
KEY_WIFI_SUMMARY = "-WIFI-SUMMARY-"
KEY_WIFI_TABLE = "-WIFI-TABLE-"
KEY_RESULT_RAW = "-RESULT-RAW-"
KEY_LOG = "-LOG-"
KEY_LOG_CLEAR = "-LOG-CLEAR-"
KEY_STATUS_BAR = "-STATUS-BAR-"
KEY_ZOOM_OUT = "-ZOOM-OUT-"
KEY_ZOOM_RESET = "-ZOOM-RESET-"
KEY_ZOOM_IN = "-ZOOM-IN-"

EVENT_TASK_DONE = "-EVENT-TASK-DONE-"
EVENT_LOG = "-EVENT-LOG-"
EVENT_SCAN_PROGRESS = "-EVENT-SCAN-PROGRESS-"
EVENT_ZOOM = "-EVENT-ZOOM-"

DEFAULT_UI_SCALE = 1.0
MIN_UI_SCALE = 1.0
MAX_UI_SCALE = 3.0
UI_SCALE_STEP = 0.1
BASE_WINDOW_WIDTH = 1180
BASE_WINDOW_HEIGHT = 760


def _scaled(value: int, scale: float) -> int:
    return max(1, int(round(value * scale)))


def _left_panel_layout() -> list[list[sg.Element]]:
    return [
        [
            sg.Frame(
                "连接参数",
                [
                    [sg.Text("设备名过滤", size=(10, 1)), sg.Input(key=KEY_TARGET_NAME, expand_x=True)],
                    [
                        sg.Text("扫描超时(s)", size=(10, 1)),
                        sg.Input(key=KEY_SCAN_TIMEOUT, size=(8, 1)),
                        sg.Text("等待超时(s)", size=(10, 1)),
                        sg.Input(key=KEY_WAIT_TIMEOUT, size=(8, 1)),
                        sg.Checkbox("详细日志", key=KEY_VERBOSE, default=False),
                    ],
                ],
                expand_x=True,
            )
        ],
        [
            sg.Frame(
                "扫描与设备",
                [
                    [
                        sg.Button("扫描", key=KEY_SCAN, size=(10, 1), button_color=("white", "#1E6F5C")),
                        sg.Text("未扫描", key=KEY_SCAN_SUMMARY, text_color="#4B5563", expand_x=True),
                    ],
                    [
                        sg.Table(
                            values=[],
                            headings=["名称", "地址", "UUID"],
                            key=KEY_DEVICE_TABLE,
                            enable_events=True,
                            auto_size_columns=False,
                            col_widths=[20, 20, 26],
                            num_rows=12,
                            expand_x=True,
                            expand_y=True,
                            justification="left",
                            select_mode=sg.TABLE_SELECT_MODE_BROWSE,
                        )
                    ],
                    [
                        sg.Text("会话：", size=(6, 1)),
                        sg.Text("未连接", key=KEY_SESSION_STATE, text_color="#B91C1C"),
                        sg.Push(),
                        sg.Button("连接", key=KEY_CONNECT, size=(10, 1), button_color=("white", "#2563EB")),
                        sg.Button("断开", key=KEY_DISCONNECT, size=(10, 1), button_color=("white", "#475569")),
                    ],
                    [
                        sg.Text("健康：", size=(6, 1)),
                        sg.Text("●", key=KEY_HEALTH_DOT, text_color="#9CA3AF", font=("PingFang SC", 16)),
                        sg.Text("连接未建立", key=KEY_HEALTH_TEXT, text_color="#6B7280"),
                    ],
                ],
                expand_x=True,
                expand_y=True,
            )
        ],
    ]


def _right_panel_layout() -> list[list[sg.Element]]:
    return [
        [
            sg.Frame(
                "Wi-Fi 配网",
                [
                    [sg.Text("SSID", size=(8, 1)), sg.Input(key=KEY_SSID, expand_x=True)],
                    [sg.Text("密码", size=(8, 1)), sg.Input(key=KEY_PASSWORD, password_char="*", expand_x=True)],
                    [
                        sg.Button("保存凭据", key=KEY_SAVE_WIFI, size=(10, 1), button_color=("white", "#0E7490")),
                        sg.Push(),
                        sg.Button("执行配网", key=KEY_PROVISION, size=(10, 1), button_color=("white", "#0F766E")),
                    ],
                ],
                expand_x=True,
            )
        ],
        [
            sg.Frame(
                "诊断命令",
                [
                    [
                        sg.Button("status", key=KEY_DIAG_STATUS, size=(10, 1)),
                        sg.Button("wifi.scan", key=KEY_DIAG_WIFI_SCAN, size=(10, 1)),
                        sg.Button("ping", key=KEY_DIAG_PING, size=(10, 1)),
                        sg.Button("help", key=KEY_DIAG_HELP, size=(10, 1)),
                    ]
                ],
                expand_x=True,
            )
        ],
        [
            sg.Frame(
                "结果",
                [
                    [
                        sg.TabGroup(
                            [
                                [
                                    sg.Tab(
                                        "概览",
                                        [
                                            [
                                                sg.Multiline(
                                                    "",
                                                    key=KEY_RESULT_OVERVIEW,
                                                    size=(60, 8),
                                                    expand_x=True,
                                                    expand_y=True,
                                                    disabled=True,
                                                    autoscroll=True,
                                                    no_scrollbar=False,
                                                )
                                            ]
                                        ],
                                        key=KEY_TAB_OVERVIEW,
                                    ),
                                    sg.Tab(
                                        "状态",
                                        [
                                            [sg.Text("暂无状态数据", key=KEY_STATUS_SUMMARY, text_color="#6B7280")],
                                            [
                                                sg.Table(
                                                    values=[],
                                                    headings=["项", "值"],
                                                    key=KEY_STATUS_TABLE,
                                                    auto_size_columns=False,
                                                    col_widths=[16, 46],
                                                    num_rows=8,
                                                    expand_x=True,
                                                    expand_y=True,
                                                    justification="left",
                                                )
                                            ]
                                        ],
                                        key=KEY_TAB_STATUS,
                                    ),
                                    sg.Tab(
                                        "Wi-Fi 扫描",
                                        [
                                            [sg.Text("暂无 Wi-Fi 扫描数据", key=KEY_WIFI_SUMMARY, text_color="#4B5563")],
                                            [
                                                sg.Table(
                                                    values=[],
                                                    headings=["SSID", "信号", "频道"],
                                                    key=KEY_WIFI_TABLE,
                                                    auto_size_columns=False,
                                                    col_widths=[30, 16, 8],
                                                    num_rows=8,
                                                    expand_x=True,
                                                    expand_y=True,
                                                    justification="left",
                                                )
                                            ],
                                        ],
                                        key=KEY_TAB_WIFI,
                                    ),
                                    sg.Tab(
                                        "原始",
                                        [
                                            [
                                                sg.Multiline(
                                                    "",
                                                    key=KEY_RESULT_RAW,
                                                    size=(60, 8),
                                                    expand_x=True,
                                                    expand_y=True,
                                                    disabled=True,
                                                    autoscroll=True,
                                                    no_scrollbar=False,
                                                )
                                            ]
                                        ],
                                        key=KEY_TAB_RAW,
                                    ),
                                ]
                            ],
                            key=KEY_RESULT_TABS,
                            expand_x=True,
                            expand_y=True,
                        )
                    ]
                ],
                expand_x=True,
                expand_y=True,
            )
        ],
        [
            sg.Frame(
                "日志",
                [
                    [sg.Push(), sg.Button("清空日志", key=KEY_LOG_CLEAR, size=(10, 1), button_color=("white", "#334155"))],
                    [
                        sg.Multiline(
                            "",
                            key=KEY_LOG,
                            size=(60, 14),
                            expand_x=True,
                            expand_y=True,
                            disabled=True,
                            autoscroll=True,
                            no_scrollbar=False,
                        )
                    ]
                ],
                expand_x=True,
                expand_y=True,
            )
        ],
    ]


def build_window(initial_state: GuiState, *, scale_factor: float = DEFAULT_UI_SCALE) -> sg.Window:
    sg.theme("NeutralBlue")
    base_font_size = _scaled(11, scale_factor)
    sg.set_options(font=("PingFang SC", base_font_size))

    layout = [
        [
            sg.Column(_left_panel_layout(), expand_x=True, expand_y=True, pad=(8, 8)),
            sg.VSeperator(),
            sg.Column(_right_panel_layout(), expand_x=True, expand_y=True, pad=(8, 8)),
        ],
        [
            sg.StatusBar(initial_state.status_text(), key=KEY_STATUS_BAR, expand_x=True),
            sg.Button("A-", key=KEY_ZOOM_OUT, size=(4, 1)),
            sg.Button("100%", key=KEY_ZOOM_RESET, size=(6, 1)),
            sg.Button("A+", key=KEY_ZOOM_IN, size=(4, 1)),
        ],
    ]
    window = sg.Window(
        "云纵科技-无线配置器",
        layout,
        resizable=True,
        finalize=True,
        return_keyboard_events=False,
    )
    window.set_min_size((680, 460))
    try:
        window.TKroot.geometry(f"{_scaled(BASE_WINDOW_WIDTH, scale_factor)}x{_scaled(BASE_WINDOW_HEIGHT, scale_factor)}")
    except Exception:
        pass
    return window


def get_base_tk_scaling(window: sg.Window) -> float:
    try:
        return float(window.TKroot.tk.call("tk", "scaling"))
    except Exception:
        return 1.0


def apply_ui_scale(window: sg.Window, *, base_scaling: float, scale_factor: float) -> float:
    clamped = max(MIN_UI_SCALE, min(MAX_UI_SCALE, scale_factor))

    def _apply_font_recursive(widget: Any, size: int) -> None:
        try:
            widget.configure(font=("PingFang SC", size))
        except Exception:
            pass
        try:
            children = widget.winfo_children()
        except Exception:
            children = []
        for child in children:
            _apply_font_recursive(child, size)

    try:
        window.TKroot.tk.call("tk", "scaling", base_scaling * clamped)
        _apply_font_recursive(window.TKroot, _scaled(11, clamped))
    except Exception:
        return clamped
    return clamped


def fit_window_to_scale(window: sg.Window, scale_factor: float) -> None:
    width = _scaled(BASE_WINDOW_WIDTH, scale_factor)
    height = _scaled(BASE_WINDOW_HEIGHT, scale_factor)
    try:
        screen_w = int(window.TKroot.winfo_screenwidth())
        screen_h = int(window.TKroot.winfo_screenheight())
    except Exception:
        screen_w = width
        screen_h = height

    max_w = max(720, screen_w - 80)
    max_h = max(520, screen_h - 120)
    target_w = min(width, max_w)
    target_h = min(height, max_h)
    try:
        window.TKroot.geometry(f"{target_w}x{target_h}")
        window.refresh()
    except Exception:
        return


def bind_zoom_shortcuts(window: sg.Window) -> None:
    def _emit(delta: int) -> None:
        window.write_event_value(EVENT_ZOOM, delta)

    def _in_handler(_event: object) -> str:
        _emit(+1)
        return "break"

    def _out_handler(_event: object) -> str:
        _emit(-1)
        return "break"

    sequences_in = (
        "<Control-plus>",
        "<Control-equal>",
        "<Control-KP_Add>",
        "<Control-KeyPress-plus>",
        "<Control-KeyPress-equal>",
        "<Command-plus>",
        "<Command-equal>",
        "<Command-KP_Add>",
    )
    sequences_out = (
        "<Control-minus>",
        "<Control-KP_Subtract>",
        "<Control-KeyPress-minus>",
        "<Command-minus>",
        "<Command-KP_Subtract>",
    )
    try:
        for seq in sequences_in:
            window.TKroot.bind(seq, _in_handler, add="+")
        for seq in sequences_out:
            window.TKroot.bind(seq, _out_handler, add="+")
    except Exception:
        return


def update_control_states(window: sg.Window, state: GuiState) -> None:
    def _safe_update(key: str, **kwargs: Any) -> None:
        element = cast(Any, window[key])
        if element is None:
            return
        element.update(**kwargs)

    _safe_update(KEY_SCAN, disabled=not state.can_scan())
    _safe_update(KEY_CONNECT, disabled=not state.can_connect())
    _safe_update(KEY_DISCONNECT, disabled=not state.can_disconnect())
    _safe_update(KEY_PROVISION, disabled=not state.can_provision())

    diag_disabled = not state.can_run_diagnostic()
    for key in (KEY_DIAG_STATUS, KEY_DIAG_WIFI_SCAN, KEY_DIAG_PING, KEY_DIAG_HELP):
        _safe_update(key, disabled=diag_disabled)

    session_text = "已连接" if state.session_connected else "未连接"
    session_color = "#047857" if state.session_connected else "#B91C1C"
    _safe_update(KEY_SESSION_STATE, value=session_text, text_color=session_color)
    if state.session_connected:
        _safe_update(KEY_HEALTH_DOT, value="●", text_color="#16A34A")
        _safe_update(KEY_HEALTH_TEXT, value="连接健康", text_color="#166534")
    else:
        _safe_update(KEY_HEALTH_DOT, value="●", text_color="#9CA3AF")
        _safe_update(KEY_HEALTH_TEXT, value="连接未建立", text_color="#6B7280")
    _safe_update(KEY_STATUS_BAR, value=state.status_text())
