use libshizen::{
    entities::{Note, NoteId, PeerId, PeerInfo},
    storage::TodoStorage,
    DefaultStorage, ShizenResult, SyncServer
};
use std::sync::Mutex;
use std::net::SocketAddr;
use std::cell::RefCell;
use std::thread_local;
use serde::{Deserialize, Serialize};

// Define our own SyncResults struct that is serializable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResults {
    pub updated_peer_clock: usize,
    pub num_changes: usize,
}

// Since RusqliteStorage isn't thread-safe due to Rc<RefCell<>>, we'll use a thread-local storage approach
thread_local! {
    static STORAGE: RefCell<Option<DefaultStorage>> = RefCell::new(None);
}

// This struct is just a marker for our state
pub struct AppState {
    sync_server: Mutex<Option<SyncServer>>,
}

impl AppState {
    pub fn new() -> Self {
        // Initialize the thread-local storage
        STORAGE.with(|storage| {
            let mut storage = storage.borrow_mut();
            if storage.is_none() {
                *storage = Some(DefaultStorage::open(None).expect("Failed to open storage"));
            }
        });
        
        Self {
            sync_server: Mutex::new(None),
        }
    }
    
    // Helper method to run operations on the storage
    fn with_storage<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&DefaultStorage) -> ShizenResult<R>,
    {
        STORAGE.with(|storage| {
            let storage = storage.borrow();
            let storage = storage.as_ref().ok_or_else(|| "Storage not initialized".to_string())?;
            f(storage).map_err(|e| e.to_string())
        })
    }
}

// Commands for note operations
#[tauri::command]
fn load_all_notes(state: tauri::State<'_, AppState>) -> Result<Vec<Note>, String> {
    state.with_storage(|storage| storage.load_all_notes())
}

#[tauri::command]
fn load_all_unblocked_notes(
    state: tauri::State<'_, AppState>,
    load_completed: bool,
) -> Result<Vec<Note>, String> {
    state.with_storage(|storage| storage.load_all_unblocked_notes(load_completed))
}

#[tauri::command]
fn create_new_note(
    state: tauri::State<'_, AppState>,
    title: String,
    description: Option<String>,
    parent: Option<String>,
) -> Result<Note, String> {
    let parent_id = match parent {
        Some(id) => Some(id.parse::<NoteId>().map_err(|e| e.to_string())?),
        None => None,
    };
    
    let parent_id_ref = parent_id.as_ref();
    
    state.with_storage(|storage| storage.create_new_note(
        &title,
        description.as_deref(),
        parent_id_ref,
    ))
}

#[tauri::command]
fn load_note(state: tauri::State<'_, AppState>, note_id: String) -> Result<Note, String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.load_note(&note_id))
}

#[tauri::command]
fn update_title(
    state: tauri::State<'_, AppState>,
    note_id: String,
    title: String,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.update_title(&note_id, &title))
}

#[tauri::command]
fn update_description(
    state: tauri::State<'_, AppState>,
    note_id: String,
    description: Option<String>,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.update_description(&note_id, description.as_deref()))
}

#[tauri::command]
fn update_parent(
    state: tauri::State<'_, AppState>,
    note_id: String,
    parent: Option<String>,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    
    let parent_id = match parent {
        Some(id) => Some(id.parse::<NoteId>().map_err(|e| e.to_string())?),
        None => None,
    };
    
    let parent_id_ref = parent_id.as_ref();
    
    state.with_storage(|storage| storage.update_parent(&note_id, parent_id_ref))
}

#[tauri::command]
fn set_completed(
    state: tauri::State<'_, AppState>,
    note_id: String,
    completed: bool,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.set_completed(&note_id, completed))
}

#[tauri::command]
fn delete_note(state: tauri::State<'_, AppState>, note_id: String) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.delete_note(&note_id))
}

#[tauri::command]
fn add_blocked_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    blocked_note: String,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    let blocked_note = blocked_note.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.add_blocked_note(&note_id, &blocked_note))
}

#[tauri::command]
fn remove_blocked_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    blocked_note: String,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    let blocked_note = blocked_note.parse::<NoteId>().map_err(|e| e.to_string())?;
    state.with_storage(|storage| storage.remove_blocked_note(&note_id, &blocked_note))
}

#[tauri::command]
fn adjust_rank_between(
    state: tauri::State<'_, AppState>,
    note_id: String,
    before: Option<String>,
    after: Option<String>,
) -> Result<(), String> {
    let note_id = note_id.parse::<NoteId>().map_err(|e| e.to_string())?;
    
    let before_id = match before {
        Some(id) => Some(id.parse::<NoteId>().map_err(|e| e.to_string())?),
        None => None,
    };
    
    let after_id = match after {
        Some(id) => Some(id.parse::<NoteId>().map_err(|e| e.to_string())?),
        None => None,
    };
    
    let before_id_ref = before_id.as_ref();
    let after_id_ref = after_id.as_ref();
    
    state.with_storage(|storage| storage.adjust_rank_between(&note_id, after_id_ref, before_id_ref))
}

// Commands for sync operations
#[tauri::command]
fn get_peers(state: tauri::State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    state.with_storage(|storage| storage.get_peers())
}

#[tauri::command]
fn add_peer(
    state: tauri::State<'_, AppState>,
    addr: String,
    local_server_port: Option<u16>,
) -> Result<String, String> {
    let socket_addr = addr.parse::<SocketAddr>().map_err(|e| e.to_string())?;
    
    let local_server_addr = local_server_port.map(|port| {
        let mut local_addr = socket_addr;
        local_addr.set_port(port);
        local_addr
    });
    
    let local_server_addr_ref = local_server_addr.as_ref();
    
    let peer_id = state.with_storage(|storage| 
        storage.add_peer(&socket_addr, local_server_addr_ref)
    )?;
    
    Ok(peer_id.0.to_string())
}

#[tauri::command]
fn remove_peer(state: tauri::State<'_, AppState>, peer_id: String) -> Result<(), String> {
    let uuid = peer_id.parse::<uuid::Uuid>().map_err(|e| e.to_string())?;
    let peer_id = PeerId(uuid);
    
    state.with_storage(|storage| storage.remove_peer(&peer_id))
}

#[tauri::command]
fn sync_with_peer(
    state: tauri::State<'_, AppState>,
    peer_id: String,
) -> Result<SyncResults, String> {
    let uuid = peer_id.parse::<uuid::Uuid>().map_err(|e| e.to_string())?;
    let peer_id = PeerId(uuid);
    
    // We need to get the peer info first
    let peer_info = state.with_storage(|storage| storage.get_peer(&peer_id))?;
    
    // We need the clock before sync
    let before_clock = state.with_storage(|storage| storage.get_clock())?;
    
    // Then perform sync
    state.with_storage(|storage| storage.sync_with_peer(&peer_info))?;
    
    // Get the peer info after sync to check the clock
    let after_info = state.with_storage(|storage| storage.get_peer(&peer_id))?;
    
    // Calculate the number of changes based on the clock difference
    let changes = after_info.clock - before_clock;
    
    // Create our serializable SyncResults
    Ok(SyncResults {
        updated_peer_clock: after_info.clock,
        num_changes: changes,
    })
}

#[tauri::command]
fn start_sync_server(
    state: tauri::State<'_, AppState>,
    port: u16,
) -> Result<(), String> {
    let mut sync_server_guard = state.sync_server.lock().map_err(|e| e.to_string())?;
    
    if sync_server_guard.is_some() {
        return Err("Sync server already running".to_string());
    }
    
    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().map_err(|e| e.to_string())?;
    
    // We'll use the current directory as a fallback
    let storage_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    
    let server = SyncServer::listen_on_thread(addr, storage_dir).map_err(|e| e.to_string())?;
    
    *sync_server_guard = Some(server);
    
    Ok(())
}

// Commands for undo/redo operations
#[tauri::command]
fn undo(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    state.with_storage(|storage| storage.undo())
}

#[tauri::command]
fn redo(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    state.with_storage(|storage| storage.redo())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            load_all_notes,
            load_all_unblocked_notes,
            create_new_note,
            load_note,
            update_title,
            update_description,
            update_parent,
            set_completed,
            delete_note,
            add_blocked_note,
            remove_blocked_note,
            adjust_rank_between,
            get_peers,
            add_peer,
            remove_peer,
            sync_with_peer,
            start_sync_server,
            undo,
            redo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}