/// tray.rs
/// 系统托盘图标模块
///
/// 管理托盘图标状态切换和菜单构建
/// 使用 tray-icon crate 实现

use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
use tracing::{debug, info};

use crate::health::GatewayStatus;

/// 托盘菜单项 ID 常量
pub const MENU_OPEN: &str        = "menu_open";
pub const MENU_SETTINGS: &str    = "menu_settings";
pub const MENU_RESTART: &str     = "menu_restart";
pub const MENU_AUTOSTART: &str   = "menu_autostart";
pub const MENU_EXPORT_LOGS: &str = "menu_export_logs";
pub const MENU_QUIT: &str        = "menu_quit";

/// 托盘管理器
///
/// 持有托盘图标和菜单项的引用
/// 菜单项字段需要保持 alive，否则会被 Drop 导致菜单项消失
#[allow(dead_code)]
pub struct TrayManager {
    tray: TrayIcon,

    /// 菜单项引用（持有以延长生命周期，不需要外部读取）
    pub item_open: MenuItem,
    pub item_settings: MenuItem,
    pub item_restart: MenuItem,
    pub item_autostart: MenuItem,
    pub item_export_logs: MenuItem,
    pub item_quit: MenuItem,
}

impl TrayManager {
    /// 创建托盘图标和菜单
    pub fn new(autostart_enabled: bool) -> Result<Self> {
        // 构建菜单
        let item_open     = MenuItem::with_id(MENU_OPEN,        "打开龙虾主页",    true, None);
        let item_settings = MenuItem::with_id(MENU_SETTINGS,    "设置 / 修改配置", true, None);
        let item_restart  = MenuItem::with_id(MENU_RESTART,     "重启服务",        true, None);
        let item_autostart = MenuItem::with_id(
            MENU_AUTOSTART,
            if autostart_enabled { "✓ 开机自启" } else { "  开机自启" },
            true,
            None,
        );
        let item_export_logs = MenuItem::with_id(MENU_EXPORT_LOGS, "导出日志到桌面", true, None);
        let item_quit = MenuItem::with_id(MENU_QUIT, "退出", true, None);

        let menu = Menu::new();
        menu.append(&item_open)?;
        menu.append(&item_settings)?;
        menu.append(&item_restart)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&item_autostart)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&item_export_logs)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&item_quit)?;

        // 简化版：使用静态图标，不显示状态
        let icon = load_icon(GatewayStatus::Running)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("AI 助手")
            .with_icon(icon)
            .build()?;

        info!("托盘图标创建成功");

        Ok(Self {
            tray,
            item_open,
            item_settings,
            item_restart,
            item_autostart,
            item_export_logs,
            item_quit,
        })
    }

    /// 根据 Gateway 状态更新托盘图标和 tooltip
    pub fn update_status(&mut self, status: &GatewayStatus) -> Result<()> {
        debug!("更新托盘状态: {:?}", status);

        let icon = load_icon(status.clone())?;
        self.tray.set_icon(Some(icon))?;
        self.tray.set_tooltip(Some(status.tooltip()))?;

        Ok(())
    }

    /// 更新"开机自启"菜单项的勾选状态
    pub fn update_autostart_item(&self, enabled: bool) {
        let label = if enabled { "✓ 开机自启" } else { "  开机自启" };
        self.item_autostart.set_text(label);
    }
}

/// 根据状态加载对应图标
///
/// 三种状态对应不同颜色的 PNG 图标（内嵌在二进制中）
fn load_icon(status: GatewayStatus) -> Result<Icon> {
    let png_bytes: &[u8] = match status {
        GatewayStatus::Starting => include_bytes!("../assets/icon-starting.png"),
        GatewayStatus::Running  => include_bytes!("../assets/icon-running.png"),
        GatewayStatus::Stopped  => include_bytes!("../assets/icon-stopped.png"),
    };

    // 使用 image crate 解析 PNG
    let img = image::load_from_memory(png_bytes)?
        .into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    Icon::from_rgba(rgba, width, height)
        .map_err(|e| anyhow::anyhow!("创建托盘图标失败: {}", e))
}

/// 处理托盘菜单事件
///
/// 返回用户选择的操作
pub enum TrayAction {
    Open,
    Settings,
    Restart,
    ToggleAutostart,
    ExportLogs,
    Quit,
    None,
}

/// 从 MenuEvent 解析用户操作
pub fn handle_menu_event(event: MenuEvent) -> TrayAction {
    match event.id().as_ref() {
        MENU_OPEN        => TrayAction::Open,
        MENU_SETTINGS    => TrayAction::Settings,
        MENU_RESTART     => TrayAction::Restart,
        MENU_AUTOSTART   => TrayAction::ToggleAutostart,
        MENU_EXPORT_LOGS => TrayAction::ExportLogs,
        MENU_QUIT        => TrayAction::Quit,
        _                => TrayAction::None,
    }
}
