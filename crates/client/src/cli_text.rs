#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Lang {
    En,
    Zh,
}

impl Lang {
    pub(crate) fn from_cli_arg(value: &str) -> Self {
        if value.eq_ignore_ascii_case("en") {
            Self::En
        } else {
            Self::Zh
        }
    }

    pub(crate) fn t<'a>(&self, key: &'a str) -> &'a str {
        match (self, key) {
            (Lang::En, "found_conn") => "✅ Selected device: {}. Connecting...",
            (Lang::Zh, "found_conn") => "✅ 已选择设备：{}。正在建立连接...",
            (Lang::En, "handshake_ok") => "✅ Handshake complete. Characteristics discovered.",
            (Lang::Zh, "handshake_ok") => "✅ 握手通讯完成。已捕捉读写通道频道。",
            (Lang::En, "prompt_device") => "Select a device to connect:",
            (Lang::Zh, "prompt_device") => "请选择要连接的设备:",
            (Lang::En, "scan_results") => "Discovered candidate devices:",
            (Lang::Zh, "scan_results") => "发现以下候选设备:",
            (Lang::En, "scan_waiting") => "No matching gateway yet. Keep the device nearby and powered on.",
            (Lang::Zh, "scan_waiting") => "暂未发现匹配设备。请保持设备上电，并尽量靠近电脑。",
            (Lang::En, "scan_enter_to_select") => {
                "Press Enter to stop scanning now and choose from the devices above."
            }
            (Lang::Zh, "scan_enter_to_select") => {
                "按 Enter 可立即停止扫描，并从上方已发现设备中选择。"
            }
            (Lang::En, "scan_stopped_early") => "Scan stopped early by user input.",
            (Lang::Zh, "scan_stopped_early") => "已根据输入提前停止扫描。",
            (Lang::En, "single_match") => "Only one candidate found. Using it automatically.",
            (Lang::Zh, "single_match") => "仅发现一台候选设备，将自动使用该设备。",
            (Lang::En, "opt_stat") => "📡 Network / System Status",
            (Lang::Zh, "opt_stat") => "📡 获取系统及网络状态",
            (Lang::En, "opt_scan") => "🔍 Scan Wi-Fi Hotspots",
            (Lang::Zh, "opt_scan") => "🔍 扫描周边 Wi-Fi 信号",
            (Lang::En, "opt_prov") => "🔑 Provision Wi-Fi",
            (Lang::Zh, "opt_prov") => "🔑 下发核心网配网卡",
            (Lang::En, "opt_profiles") => "🧹 Manage saved Wi-Fi profiles",
            (Lang::Zh, "opt_profiles") => "🧹 管理已保存 Wi-Fi",
            (Lang::En, "opt_exit") => "🚪 Exit",
            (Lang::Zh, "opt_exit") => "🚪 退出",
            (Lang::En, "prompt_menu") => "Select an operation:",
            (Lang::Zh, "prompt_menu") => "↓请使用方向键选择需要执行的指令:",
            (Lang::En, "press_enter_return") => "Press Enter to return to the menu...",
            (Lang::Zh, "press_enter_return") => "按 Enter 返回菜单...",
            (Lang::En, "prmpt_ssid") => "Enter Target SSID:",
            (Lang::Zh, "prmpt_ssid") => "请输入需连接到的 Wi-Fi 账号 (SSID):",
            (Lang::En, "prmpt_pwd") => "Enter Wi-Fi Password (hidden):",
            (Lang::Zh, "prmpt_pwd") => "请输入 Wi-Fi 密码 (隐藏输入不显示):",
            (Lang::En, "profiles_empty") => "No saved Wi-Fi profiles were reported.",
            (Lang::Zh, "profiles_empty") => "设备未返回已保存 Wi-Fi 记忆。",
            (Lang::En, "profiles_no_deletable") => "Only active/protected profiles were found; nothing can be deleted from the CLI default flow.",
            (Lang::Zh, "profiles_no_deletable") => "只发现当前活动或受保护的 Wi-Fi 记忆；CLI 默认流程不会删除它们。",
            (Lang::En, "profiles_select_delete") => "Select saved Wi-Fi profiles to delete:",
            (Lang::Zh, "profiles_select_delete") => "请选择要删除的已保存 Wi-Fi 记忆:",
            (Lang::En, "profiles_delete_skipped") => "No profile selected; delete skipped.",
            (Lang::Zh, "profiles_delete_skipped") => "未选择任何 Wi-Fi 记忆，已跳过删除。",
            _ => key,
        }
    }

    pub(crate) fn scan_header(&self, target: &str, timeout: u64) -> String {
        match self {
            Lang::En => format!(
                "🔍 Scanning for devices with prefix '{}' for {}s...",
                target, timeout
            ),
            Lang::Zh => format!(
                "🔍 正在扫描前缀为 '{}' 的蓝牙设备（超时 {} 秒）...",
                target, timeout
            ),
        }
    }

    pub(crate) fn scan_live_status(
        &self,
        spinner: &str,
        target: &str,
        remaining: u64,
        candidates: usize,
    ) -> String {
        match self {
            Lang::En => format!(
                "{spinner} Scanning prefix '{target}' | {remaining}s left | {candidates} candidate(s)"
            ),
            Lang::Zh => format!(
                "{spinner} 正在扫描前缀 '{target}' | 剩余 {remaining} 秒 | 已发现 {candidates} 台候选设备"
            ),
        }
    }
}
