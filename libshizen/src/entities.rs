use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Note {
  pub id: NoteId,
  pub title: String,
  pub description: Option<String>,
  pub parent_id: Option<NoteId>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoteId(pub Uuid);

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
