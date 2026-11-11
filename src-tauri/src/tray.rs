//! Tray / menu bar icon: Settings, Model picker, Quit.

use crate::commands::apply_settings;
use crate::{models, settings};
use tauri::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

/// The model picker's menu items, kept so their ticks can be updated when
/// the model is changed from the settings window.
struct ModelItems(Vec<CheckMenuItem<Wry>>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Loudr", true, None::<&str>)?;

    let current = settings::current(app).model;
    let model_items: Vec<CheckMenuItem<Wry>> = models::SIZES
        .iter()
        .map(|&(id, label, size_mb)| {
            CheckMenuItem::with_id(
                app,
                format!("model:{id}"),
                format!("{label} ({size_mb} MB)"),
                true,
                id == current,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<_>>()?;
    let model_refs: Vec<&dyn IsMenuItem<Wry>> = model_items.iter().map(|i| i as _).collect();
    let model_menu = Submenu::with_items(app, "Model", true, &model_refs)?;
    app.manage(ModelItems(model_items));

    let menu = Menu::with_items(
        app,
        &[
            &settings_item,
            &model_menu,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    // macOS menu bar icons are "templates": a black shape that the system
    // recolours for light and dark menu bars. Windows uses the app icon.
    #[cfg(target_os = "macos")]
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = app.default_window_icon().expect("bundle icon missing").clone();

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Loudr")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            id => {
                if let Some(model) = id.strip_prefix("model:") {
                    // Clicking toggles the tick by itself; put the ticks back
                    // in a consistent state even if the model didn't change.
                    sync_model(app, model);
                    let mut s = settings::current(app);
                    s.model = model.to_string();
                    apply_settings(app, s);
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Tick exactly one entry in the model picker.
pub fn sync_model(app: &AppHandle, model: &str) {
    if let Some(items) = app.try_state::<ModelItems>() {
        for item in &items.0 {
            let _ = item.set_checked(item.id().as_ref() == format!("model:{model}"));
        }
    }
}

pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
