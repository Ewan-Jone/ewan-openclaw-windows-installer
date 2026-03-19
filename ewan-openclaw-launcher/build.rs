/// build.rs
/// 编译时处理：Windows 平台设置 GUI 子系统（隐藏控制台窗口）

fn main() {
    // 什么都不做——windows_subsystem 已在 main.rs 的 #![cfg_attr] 里处理
    // 不需要额外的链接参数
}
