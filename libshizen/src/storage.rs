//! Generic storage for TODOs
use std::net::ToSocketAddrs;

use crate::{
  entities::{Action, Note, NoteId, PeerId, PeerInfo},
  result::ShizenResult,
  sync::SyncResults,
};

pub mod rusqlite;

pub trait TodoStorage {
  // Create
  fn create_new_note(
    &self,
    title: &str,
    description: Option<&str>,
    parent: Option<&NoteId>,
  ) -> ShizenResult<Note>;
  fn add_peer<A: ToSocketAddrs>(
    &self,
    addr: A,
    local_server_addr: Option<A>,
  ) -> ShizenResult<PeerId>;
  fn add_connected_peer<A: ToSocketAddrs>(
    &self,
    peer_id: PeerId,
    clock: usize,
    addr: &A,
  ) -> ShizenResult<()>;

  // Read
  fn load_all_notes(&self) -> ShizenResult<Vec<Note>>;
  fn load_all_unblocked_notes(&self) -> ShizenResult<Vec<Note>>;
  fn load_all_changes_since_clock(&self, clock: usize) -> ShizenResult<Vec<Action>>;
  fn load_note(&self, note_id: &NoteId) -> ShizenResult<Note>;
  fn note_exists(&self, note_id: &NoteId) -> ShizenResult<bool>;
  fn get_all_descendents(&self, note_id: &NoteId) -> ShizenResult<Vec<Note>>;
  fn get_all_blocked(&self, note_id: &NoteId, recursive: bool) -> ShizenResult<Vec<Note>>;
  fn get_peer_id(&self) -> ShizenResult<PeerId>;
  fn get_clock(&self) -> ShizenResult<usize>;
  fn get_peers(&self) -> ShizenResult<Vec<PeerInfo>>;
  fn get_peer(&self, peer_id: &PeerId) -> ShizenResult<PeerInfo>;

  // Update
  fn update_title(&self, note_id: &NoteId, title: &str) -> ShizenResult<()>;
  fn update_description(&self, note_id: &NoteId, description: Option<&str>) -> ShizenResult<()>;
  fn update_parent(&self, note_id: &NoteId, parent: Option<&NoteId>) -> ShizenResult<()>;
  fn add_blocked_note(&self, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()>;
  fn remove_blocked_note(&self, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()>;
  fn set_peer_clock(&self, peer_id: &PeerId, clock: usize) -> ShizenResult<()>;

  fn apply_action(&self, action: &Action) -> ShizenResult<()>;

  /// Returns the value the clock was rewinded to.
  fn undo(&self) -> ShizenResult<usize>;
  fn redo(&self) -> ShizenResult<usize>;

  // Delete
  fn delete_note(&self, note_id: &NoteId) -> crate::ShizenResult<()>;

  // Sync
  fn sync_with_peer(&self, peer: &PeerInfo) -> ShizenResult<SyncResults>;
}

// TODO write down the sync strat (list of peers with their claimed sync version, request with stuff more, apply and overwrite conflicts for now, rewrite my own changes back on top)
