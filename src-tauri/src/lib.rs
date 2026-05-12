pub mod storage;

use std::sync::Mutex;
use std::time::Duration;
use storage::{
    CreateNoteInput, IndexHealth, ListNotesQuery, NoteDetail, NoteSummary, SearchNotesQuery,
    SearchResult, UpdateNoteInput,
};
use tauri::{Emitter, Manager};

pub struct AppState {
    database: Mutex<rusqlite::Connection>,
}

#[tauri::command]
fn app_health() -> &'static str {
    "ready"
}

#[tauri::command]
fn create_note(
    state: tauri::State<'_, AppState>,
    input: CreateNoteInput,
) -> Result<NoteDetail, String> {
    let mut database = state.database.lock().map_err(|error| error.to_string())?;
    storage::create_note(&mut database, input).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_notes(
    state: tauri::State<'_, AppState>,
    query: ListNotesQuery,
) -> Result<Vec<NoteSummary>, String> {
    let database = state.database.lock().map_err(|error| error.to_string())?;
    storage::list_notes(&database, query).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_note(state: tauri::State<'_, AppState>, id: String) -> Result<Option<NoteDetail>, String> {
    let database = state.database.lock().map_err(|error| error.to_string())?;
    storage::get_note(&database, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn update_note(
    state: tauri::State<'_, AppState>,
    input: UpdateNoteInput,
) -> Result<NoteDetail, String> {
    let mut database = state.database.lock().map_err(|error| error.to_string())?;
    storage::update_note(&mut database, input).map_err(|error| error.to_string())
}

#[tauri::command]
fn rename_note(
    state: tauri::State<'_, AppState>,
    id: String,
    title: String,
) -> Result<NoteSummary, String> {
    let database = state.database.lock().map_err(|error| error.to_string())?;
    storage::rename_note(&database, &id, &title).map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_note(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let database = state.database.lock().map_err(|error| error.to_string())?;
    storage::delete_note(&database, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn search_notes(
    state: tauri::State<'_, AppState>,
    query: SearchNotesQuery,
) -> Result<Vec<SearchResult>, String> {
    let database = state.database.lock().map_err(|error| error.to_string())?;
    storage::search_notes(&database, query).map_err(|error| error.to_string())
}

#[tauri::command]
fn index_health(state: tauri::State<'_, AppState>) -> Result<IndexHealth, String> {
    let database = state.database.lock().map_err(|error| error.to_string())?;
    storage::index_health(&database).map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            std::fs::create_dir_all(&data_dir)?;
            let database = storage::open_database(&data_dir.join("o-note.db"))?;
            app.manage(AppState {
                database: Mutex::new(database),
            });
            spawn_indexer(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_health,
            create_note,
            list_notes,
            get_note,
            update_note,
            rename_note,
            delete_note,
            search_notes,
            index_health
        ])
        .run(tauri::generate_context!())
        .expect("failed to run o-note");
}

fn spawn_indexer(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(250));
        let state = app_handle.state::<AppState>();
        let Ok(database) = state.database.lock() else {
            continue;
        };
        let processed = storage::process_index_jobs(&database, 25).unwrap_or(0);

        if processed > 0 {
            if let Ok(health) = storage::index_health(&database) {
                let _ = app_handle.emit("index-health", health);
            }
        }
    });
}
