use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, Window};

// Структура одного элемента истории буфера (соответствует ТЗ п. 5)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: String,
    pub r#type: String, // "text", "image", "file_path", etc.
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub is_pinned: bool,
    pub created_at: String,
}

// Общее состояние приложения
struct ClipboardState {
    history: Mutex<Vec<ClipboardItem>>,
    app_dir: PathBuf,
}

// Получение пути к директории %AppData%/SmartClipboard/
fn get_app_data_dir(app: &AppHandle) -> PathBuf {
    let base_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    fs::create_dir_all(&base_dir).ok();
    base_dir
}

// Загрузка истории из history.json
fn load_history(dir: &PathBuf) -> Vec<ClipboardItem> {
    let file_path = dir.join("history.json");
    if let Ok(data) = fs::read_to_string(file_path) {
        if let Ok(items) = serde_json::from_str(&data) {
            return items;
        }
    }
    Vec::new()
}

// Сохранение истории в history.json
fn save_history(dir: &PathBuf, history: &[ClipboardItem]) {
    let file_path = dir.join("history.json");
    if let Ok(data) = serde_json::to_string_pretty(history) {
        fs::write(file_path, data).ok();
    }
}

// Tauri-команды для фронтенда
#[tauri::command]
fn get_clipboard_history(state: State<'_, ClipboardState>) -> Vec<ClipboardItem> {
    let history = state.history.lock().unwrap();
    history.clone()
}

#[tauri::command]
fn toggle_pin(id: String, state: State<'_, ClipboardState>) {
    let mut history = state.history.lock().unwrap();
    if let Some(item) = history.iter_mut().find(|i| i.id == id) {
        item.is_pinned = !item.is_pinned;
        // Сортируем: закрепленные сверху
        history.sort_by(|a, b| b.is_pinned.cmp(&a.is_pinned));
        save_history(&state.app_dir, &history);
    }
}

#[tauri::command]
fn delete_item(id: String, state: State<'_, ClipboardState>) {
    let mut history = state.history.lock().unwrap();
    history.retain(|i| i.id != id);
    save_history(&state.app_dir, &history);
}

#[tauri::command]
fn clear_history(state: State<'_, ClipboardState>) {
    let mut history = state.history.lock().unwrap();
    // Удаляем все незакрепленные записи
    history.retain(|i| i.is_pinned);
    save_history(&state.app_dir, &history);
}

// Управление сквозным кликом сквозь прозрачную зону окна
#[tauri::command]
fn set_ignore_cursor(window: Window, ignore: bool) {
    let _ = window.set_ignore_cursor_events(ignore);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_dir = get_app_data_dir(&app_handle);
            let initial_history = load_history(&app_dir);

            app.manage(ClipboardState {
                history: Mutex::new(initial_history),
                app_dir: app_dir.clone(),
            });

            // Фоновый поток мониторинга буфера обмена
            std::thread::spawn(move || {
                let mut clipboard = match Clipboard::new() {
                    Ok(cb) => cb,
                    Err(e) => {
                        eprintln!("Ошибка инициализации буфера: {}", e);
                        return;
                    }
                };

                let mut last_text = String::new();

                loop {
                    if let Ok(current_text) = clipboard.get_text() {
                        if !current_text.is_empty() && current_text != last_text {
                            last_text = current_text.clone();

                            let state = app_handle.state::<ClipboardState>();
                            let mut history = state.history.lock().unwrap();

                            // Проверяем, нет ли уже такого текста
                            if !history.iter().any(|i| i.content.as_deref() == Some(&current_text)) {
                                let new_item = ClipboardItem {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    r#type: "text".to_string(),
                                    content: Some(current_text),
                                    file_path: None,
                                    is_pinned: false,
                                    created_at: chrono::Utc::now().to_rfc3339(),
                                };

                                history.insert(0, new_item);

                                // Циклический лимит (храним максимум 50 незакрепленных)
                                if history.len() > 50 {
                                    if let Some(pos) = history.iter().rposition(|i| !i.is_pinned) {
                                        history.remove(pos);
                                    }
                                }

                                save_history(&state.app_dir, &history);
                                println!("Новый текст сохранен в историю и JSON!");
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            toggle_pin,
            delete_item,
            clear_history,
            set_ignore_cursor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}