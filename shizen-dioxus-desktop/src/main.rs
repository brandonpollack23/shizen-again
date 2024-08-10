//! Note: all assets (tailwind.css/main.css/etc.) must be prefixed with the workspace path.
#![allow(non_snake_case)]

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info, Level};
use libshizen::{entities::Note, storage::TodoStorage};

// TODO note view
// TODO settings view (with other sync hosts)
// TODO pull or push button to refresh list view
// TODO animations

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
  #[route("/")]
  NoteListView,
}

type TodoStorageSignal = Signal<Box<dyn TodoStorage>>;

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

    info!("Attempting to open/create database at path {default_storage_path:?}");

    // TODO combine open with create with a flag for creation param.
    std::fs::create_dir_all(default_storage_path.parent().unwrap()).unwrap();
    let rusqlitedb = libshizen::DefaultStorage::open_create(Some(&default_storage_path), false);
    if let Err(e) = rusqlitedb {
      info!("Error Creating database: {e:?}");
      std::process::exit(1);
    } else {
      // TODO print error using the error pattern documented on site.
      info!("New Database created");
    }

    let db: Box<dyn TodoStorage> = Box::new(rusqlitedb.unwrap());
    Signal::new(db)
  });

  rsx! {
    div { class: "bg-slate-200 h-screen w-screen", HomeburgerLayout { Router::<Route> {} } }
  }
}

#[component]
fn HomeburgerLayout(children: Element) -> Element {
  let mut is_open = use_signal(|| false);

  rsx! {
    // TODO https://chatgpt.com/share/a35e9f13-ea58-43ed-962e-9fb4f7b2d753
    div { class: "flex flex-row h-full min-w-fit",
      // invisible column with hamburger at the top.
      div {
        class: "bg-indigo-300 pl-[.5] pr-[.5] transition-transform transform-gpu",
        class: if is_open() { "translate-x-full" } else { "translate-x-0" },

        div {
          img {
            class: "min-h-8 min-w-8 w-8 h-8",
            src: "res/hamburger.svg",
            alt: "Hamburger menu",
            onclick: move |_| *is_open.write() = !is_open()
          }
        }
      }
      {children}
    }
  }
}

// TODO indentation by parent
// TODO custom ordering.
// TODO toggle to show things blocking this note.
// TODO blocked by with toggle in settings or sidebar.
// TODO expandable graveyard of completed notes
// TODO ghosting completed parents.
#[component]
fn NoteListView() -> Element {
  // TODO show_blocked flag
  // TODO async load all unblocked notes in a coroutine or something.
  let db = use_database();
  let todos = use_memo(move || db.read().load_all_unblocked_notes(false).unwrap());

  // TODO https://github.com/DioxusLabs/dioxus/blob/main/examples/todomvc.rs use this example, instead of hashmap use my database.

  rsx! {
    if !todos().is_empty() {
      ul { class: "bg-slate-50 w-full",
        for note in &todos() {
          li { key: "{note.id}",
            TodoListItem { db, note: note.clone() }
          }
        }
      }
    }
  }
}

#[component]
fn TodoListItem(db: TodoStorageSignal, note: ReadOnlySignal<Note>) -> Element {
  let db = use_database();

  let blocklist_str = note
    .read()
    .notes_this_blocks
    .iter()
    .map(|n| n.0.to_string())
    .collect::<Vec<_>>()
    .join(", ");

  rsx! {
    div { class: "flex flex-row mb-3 p-2 shadow cursor-grab w-full items-center",
      // Notes async event handlers also exist (dioxus provides async method)
      input {
        class: "self-center mr-3 w-6 h-6 hover:cursor-pointer",
        onclick: move |_| toggle_note_complete(db, note),
        r#type: "checkbox"
      }
      div { class: "overflow-hidden flex-grow min-w-0",
        div {
          p { class: "font-bold text-ellipsis whitespace-nowrap", "{note.read().title}" }
        }
        if note.read().description.is_some() {
          div { class: "text-ellipsis italic whitespace-nowrap",
            "{note.read().description.as_ref().unwrap()}"
          }
        }
        if !note.read().notes_this_blocks.is_empty() {
          div { class: "text-ellipsis whitespace-nowrap italic",
            span { "Blocking: [" }
            // TODO instead of a blocklist string use a title.
            span { "{blocklist_str}" }
            span { "]" }
          }
        }
      }
      // TODO when description overflows it hides the details button and there is no ellipses.
      img {
        class: "h-8 w-8 min-h-8 min-w-8 p-1 ml-auto flex-shrink-0 hover:cursor-pointer",
        src: "res/details.svg",
        alt: "Open note details button",
        onclick: move |_| todo!()
      }
    }
  }
}

fn use_database() -> TodoStorageSignal {
  use_context()
}

fn toggle_note_complete(mut db: TodoStorageSignal, note: ReadOnlySignal<Note>) {
  let note = note.read();
  info!("Marking note {:?} completed: {}", note.id, !note.completed);

  if let Err(e) = db.write().set_completed(&note.id, !note.completed) {
    error!(
      "Error marking note {} as {}:\n\t{:#?}",
      note.id.0.to_string(),
      !note.completed,
      e
    );
  }
}
