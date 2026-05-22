pub mod model;
pub mod desktop;
pub mod runner;
pub mod dpkg;
pub mod sizes;
pub mod snapd;
pub mod sources;
pub mod icons;
pub mod aggregate;
pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::list_apps])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod harness_smoke {
    #[test]
    fn harness_runs() {
        assert_eq!(2 + 2, 4);
    }
}
