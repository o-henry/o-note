pub mod storage;

#[tauri::command]
fn app_health() -> &'static str {
    "ready"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_health])
        .run(tauri::generate_context!())
        .expect("failed to run o-note");
}
