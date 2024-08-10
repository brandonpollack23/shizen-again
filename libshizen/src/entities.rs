use std::{fmt::Display, net::SocketAddr, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Note {
  pub id: NoteId,
  pub title: String,
  pub description: Option<String>,
  pub completed: bool,
  pub parent_id: Option<NoteId>,
  pub children_ids: Vec<NoteId>,
  pub notes_this_blocks: Vec<NoteId>,
  pub notes_blocking_this: Vec<NoteId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoteId(pub Uuid);

impl FromStr for NoteId {
  type Err = uuid::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(NoteId(Uuid::from_str(s)?))
  }
}

impl Display for NoteId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0.to_string())
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerId(pub Uuid);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerInfo {
  pub peer_id: PeerId,
  pub clock: usize,
  pub addr: SocketAddr,
}

impl From<Uuid> for NoteId {
  fn from(value: Uuid) -> Self {
    NoteId(value)
  }
}

impl From<Uuid> for PeerId {
  fn from(value: Uuid) -> Self {
    PeerId(value)
  }
}

impl From<NoteId> for Uuid {
  fn from(value: NoteId) -> Self {
    value.0
  }
}

impl From<PeerId> for Uuid {
  fn from(value: PeerId) -> Self {
    value.0
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
  CreateNote {
    id: NoteId,
    title: String,
    description: Option<String>,
    parent: Option<NoteId>,
  },
  SetCompleted {
    id: NoteId,
    new_completed: bool,
    old_completed: bool,
  },
  UpdateTitle {
    id: NoteId,
    old_title: String,
    new_title: String,
  },
  UpdateDescription {
    id: NoteId,
    old_description: Option<String>,
    new_description: Option<String>,
  },
  ChangeParent {
    id: NoteId,
    old_parent: Option<NoteId>,
    new_parent: Option<NoteId>,
  },
  AddDependency {
    blocker: NoteId,
    blockee: NoteId,
  },
  RemoveDependency {
    blocker: NoteId,
    blockee: NoteId,
  },
  DeleteNote {
    note: Note,
  },
}
