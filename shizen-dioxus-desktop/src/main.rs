//! Note: all assets (tailwind.css/main.css/etc.) must be prefixed with the workspace path.
#![allow(non_snake_case)]

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use libshizen::{entities::Note, storage::TodoStorage};

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
  #[route("/")]
  NoteListView,
}

fn main() {
  // Init logger
  dioxus_logger::init(Level::INFO).expect("failed to init logger");
  info!("starting shizen desktop app...");

  let cfg = dioxus::desktop::Config::new().with_custom_head(
    r#"<link rel="stylesheet" href="shizen-dioxus-desktop/tailwind.css">"#.to_string(),
  );
  LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[component]
fn App() -> Element {
  use_context_provider(|| {
    // TODO have settings object that contains selected database path.
    // let default_storage_path = format!("{}/shizen.db", env!("HOME"));
    let default_storage_path: PathBuf = "testdb/test.db".into();

    info!("Attempting to create database at path {default_storage_path:?}");

    // TODO combine open with create with a flag for creation param.
    std::fs::create_dir_all(&default_storage_path.parent().unwrap()).unwrap();
    let rusqlitedb = libshizen::DefaultStorage::open_create(Some(&default_storage_path), false);
    if let Err(e) = rusqlitedb {
      info!("Error Creating database: {e:?}");
      std::process::exit(1);
    } else {
      info!("New Database created");
    }

    let db: Box<dyn TodoStorage> = Box::new(rusqlitedb.unwrap());

    return Signal::new(db);
  });

  rsx! {
    div { class: "bg-slate-300 h-screen", Router::<Route> {} }
  }
}

// TODO parent sorting.
// TODO custom ordering.
// TODO toggle to show things blocking this note.
// TODO blocked by with toggle in settings or sidebar.
#[component]
fn NoteListView() -> Element {
  // TODO show_blocked

  let db = use_context::<Signal<Box<dyn TodoStorage>>>();
  // TODO async load all unblocked notes in a coroutine or something.

  rsx! {
    if let Ok(unblocked_notes) = db.read().load_all_unblocked_notes() {
      if unblocked_notes.len() > 0 {
        ul { class: "bg-slate-50",
          for note in unblocked_notes {
            li {
              TodoListItem { note }
            }
          }
        }
      }
    } else {
      div { class: "text-red", "Error Loading Notes!" }
    }
  }
}

#[component]
fn TodoListItem(note: Note) -> Element {
  let blocklist_str = note
    .notes_this_blocks
    .iter()
    .map(|n| n.0.to_string())
    .collect::<Vec<_>>()
    .join(", ");

  rsx! {
    div { class: "flex flex-row mb-3 p-2 shadow cursor-grab",
      input { class: "self-center mr-3 w-4 h-4", r#type: "checkbox" }
      div {
        div {
          p { class: "font-bold", "{note.title}" }
        }
        if note.description.is_some() {
          div { class: "italic", "{note.description.unwrap()}" }
        }
        if note.notes_this_blocks.len() > 0 {
          div { class: "italic",
            span { "Blocking: [" }
            span { class: "text-ellipsis", "{blocklist_str}" }
            span { "]" }
          }
        }
      }
    }
  }
}
