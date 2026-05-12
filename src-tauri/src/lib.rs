pub mod storage;

use std::sync::Mutex;
use storage::{CreateNoteInput, ListNotesQuery, NoteDetail, NoteSummary, UpdateNoteInput};
use tauri::Manager;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_health,
            create_note,
            list_notes,
            get_note,
            update_note,
            rename_note,
            delete_note
        ])
        .run(tauri::generate_context!())
        .expect("failed to run o-note");
}
