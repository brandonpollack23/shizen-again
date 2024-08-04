use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Note {
  pub id: NoteId,
  pub title: String,
  pub description: Option<String>,
  pub parent_id: Option<NoteId>,
  pub children_ids: Vec<NoteId>,
  pub notes_this_blocks: Vec<NoteId>,
  pub notes_blocking_this: Vec<NoteId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoteId(pub Uuid);

impl Display for NoteId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0.to_string())
  }
}

#[derive(Debug, Clone)]
pub struct PeerId(pub Uuid);

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
pub(crate) enum Actions {
  CreateNote {
    id: NoteId,
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
