//! Tauri 应用入口：菜单栏 tray + 全局快捷键 + 占位窗口 + state 注入。
//! 这是 lib crate，main.rs 调它。

mod commands;
mod db;
mod llm;
mod stt;

use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

use commands::AppState;
use db::Db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Life-Log starting up...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // 1. 初始化数据目录（macOS: ~/Library/Application Support/com.crd.life-log/data）
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取 app_data_dir")
                .join("data");
            tracing::info!("数据目录: {}", data_dir.display());

            // 2. 打开数据库
            let db = Arc::new(Db::open(&data_dir).expect("数据库初始化失败"));
            tracing::info!("数据库就绪：{}", db.path.display());

            // 3. 注入 state
            app.manage(AppState { db: db.clone() });

            // 4. 构建菜单栏 tray menu —— SPEC §5.1
            //    用 MenuBuilder 一次性构建，避免 MenuItem 变量被 drop 导致菜单失效
            let handle = app.handle();
            let menu = MenuBuilder::new(handle)
                .item(&MenuItemBuilder::with_id("checkin", "🎤 现在打卡").accelerator("CmdOrCtrl+Shift+L").build(handle)?)
                .item(&MenuItemBuilder::with_id("add_item", "📦 添加物品").accelerator("CmdOrCtrl+Shift+I").build(handle)?)
                .item(&MenuItemBuilder::with_id("add_task", "✅ 添加任务").accelerator("CmdOrCtrl+Shift+T").build(handle)?)
                .item(&MenuItemBuilder::with_id("note", "📝 随手记").accelerator("CmdOrCtrl+Shift+N").build(handle)?)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&MenuItemBuilder::with_id("review", "📊 回顾").build(handle)?)
                .item(&MenuItemBuilder::with_id("items", "📦 物品库").build(handle)?)
                .item(&MenuItemBuilder::with_id("tasks", "✅ 任务").build(handle)?)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&MenuItemBuilder::with_id("settings", "⚙️  设置").build(handle)?)
                .item(&MenuItemBuilder::with_id("pause", "⏸  暂停打卡 1 小时").build(handle)?)
                .item(&MenuItemBuilder::with_id("export", "📤 导出数据").build(handle)?)
                .item(&PredefinedMenuItem::separator(handle)?)
                .item(&MenuItemBuilder::with_id("quit", "退出").build(handle)?)
                .build()?;

            // 5. 创建 tray icon
            //    用 include_image! 宏，编译期把 PNG 解码为 RGBA（路径相对于 Cargo.toml）
            //    macOS template image 模式下，图标会按系统主题自动反色
            let tray_icon = tauri::include_image!("icons/tray.png");

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon)
                .icon_as_template(true)
                .tooltip("Life-Log（左键打开主窗口；右键弹菜单）")
                .menu(&menu)
                .show_menu_on_left_click(false)  // 左键不弹菜单，自己处理
                .on_menu_event(move |app, event| {
                    tracing::info!("菜单点击: {}", event.id.as_ref());
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "checkin" => show_checkin_window(app),
                        "settings" => show_settings_window(app),
                        other => tracing::info!("菜单 '{}' 尚未实现", other),
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键 up 时打开主窗口；右键由系统自己弹菜单
                    use tauri::tray::MouseButton::*;
                    use tauri::tray::MouseButtonState::*;
                    if let TrayIconEvent::Click { button: Left, button_state: Up, .. } = event {
                        tracing::info!("tray 左键 → 打开主窗口");
                        let app = tray.app_handle();
                        show_main_window(app);
                    }
                })
                .build(app)?;

            tracing::info!("tray 已注册");

            // 6. 启动时打卡窗口默认隐藏（在 tauri.conf.json 里 visible: false）
            //    被点击/快捷键唤起时再显示
            //    第一版没有自动打卡定时器，后续在 scheduler.rs 加

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::create_event,
            commands::update_event_structured,
            commands::list_recent_events,
            commands::get_setting,
            commands::set_setting,
            commands::llm_presets,
            commands::llm_test_connection,
            commands::llm_extract_checkin,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri run failed");
}

/// 显示主窗口；如果窗口已经被销毁（用户点了红叉），重新创建一个
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        return;
    }
    // 窗口被关掉了，重新创建
    let result = tauri::WebviewWindowBuilder::new(
        app,
        "main",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Life-Log")
    .inner_size(720.0, 540.0)
    .resizable(true)
    .center()
    .build();
    if let Err(e) = result {
        tracing::error!("重建主窗口失败: {e}");
    }
}

/// 显示打卡窗口（已存在则 focus，否则报错——窗口在 tauri.conf.json 静态声明）
fn show_checkin_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("checkin") {
        let _ = win.show();
        let _ = win.set_focus();
        // 移到屏幕右下角
        if let Ok(Some(monitor)) = win.current_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let win_w = (360.0 * scale) as i32;
            let win_h = (280.0 * scale) as i32;
            let x = (size.width as i32) - win_w - (20.0 * scale) as i32;
            let y = (size.height as i32) - win_h - (60.0 * scale) as i32;
            let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        }
    } else {
        tracing::warn!("checkin window not found");
    }
}

/// 显示/创建设置窗口
fn show_settings_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    // 动态创建
    let result = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Life-Log · 设置")
    .inner_size(560.0, 520.0)
    .resizable(true)
    .center()
    .initialization_script("window.location.hash = '#/settings';")
    .build();
    if let Err(e) = result {
        tracing::error!("创建设置窗口失败: {e}");
    }
}
