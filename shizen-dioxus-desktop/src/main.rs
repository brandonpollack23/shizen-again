//! Note: all assets (tailwind.css/main.css/etc.) must be prefixed with the workspace path.
#![allow(non_snake_case)]

use std::path::PathBuf;

use dioxus::{
  desktop::{LogicalSize, WindowBuilder},
  prelude::*,
};
use dioxus_logger::tracing::{error, info, Level};
use libshizen::{
  entities::{Note, NoteId},
  storage::TodoStorage,
};

// TODO note view
// TODO back button
// TODO settings view (with other sync hosts)
// TODO pull or push button to refresh list view
// TODO setting to enable/disable markdown rendering.
// TODO animations

// TODO navbar(column) needs to be changed to work around route https://dioxuslabs.com/learn/0.5/router/example/full-code
#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
  #[route("/")]
  NoteListView,
  #[route("/note/:note_id")]
  NoteView { note_id: NoteId },
}

type TodoStorageSignal = Signal<Box<dyn TodoStorage>>;

fn main() {
  // Init logger
  dioxus_logger::init(Level::INFO).expect("failed to init logger");
  info!("starting shizen desktop app...");

  let cfg = dioxus::desktop::Config::new()
    .with_custom_head(
      r#"<link rel="stylesheet" href="shizen-dioxus-desktop/tailwind.css">"#.to_string(),
    )
    .with_window(
      // TODO persist this stuff in the settings and remember it.
      WindowBuilder::new()
        .with_title("Shizen")
        .with_inner_size(LogicalSize::new(800, 600)),
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
        class: "mr-3 w-6 h-6 hover:cursor-pointer",
        onclick: move |_| toggle_note_complete(db, note),
        r#type: "checkbox"
      }
      div { class: "flex-grow overflow-hidden min-w-0",
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
      Link {
        to: Route::NoteView {
            note_id: note.read().id.clone(),
        },
        img {
          class: "h-8 w-8 min-h-8 min-w-8 p-1 ml-auto flex-shrink-0 hover:cursor-pointer",
          src: "res/details.svg",
          alt: "Open note details button"
        }
      }
    }
  }
}

#[component]
fn NoteView(note_id: ReadOnlySignal<NoteId>) -> Element {
  let mut db = use_database();
  let note = use_memo(move || {
    db.read()
      .load_note(&note_id.read())
      .expect("Could not load note")
  });
  let notes_this_blocks = &note.read().notes_this_blocks;
  let notes_blocking_this = &note.read().notes_blocking_this;
  let children = &note.read().children_ids;

  // TODO replace inputs with regular text areas and get the values with ids or events.
  rsx! {
    div { class: "p-2 pl-3 bg-slate-50 min-w-full",
      div { class: "flex items-center space-x-4",
        input {
          class: "w-6 h-6 hover:cursor-pointer",
          // onclick: move |_| toggle_note_complete(db, note.clone().into()),
          checked: note.read().completed,
          r#type: "checkbox"
        }
        input {
          class: "text-2xl",
          contenteditable: true,
          oninput: move |ev| {
              db.write().update_title(&note_id.read(), &ev.data.value()).unwrap()
          },
          value: "{note.read().title}"
        }
      }

      br {}

      if note.read().description.is_some() {
        p { class: "text-xl", "Description:" }
        textarea { class: "min-w-80", value: "{note.read().description.as_ref().unwrap()}" }
      }

      br {}

      p { class: "text-2xl", "Relations" }

      if note.read().parent_id.as_ref().is_some() {
        // TODO link this
        p { class: "text-xl", "Parent:" }
        a { "{note.read().parent_id.as_ref().unwrap().0.to_string()}" }
      }

      if !children.is_empty() {
        br {}
        p { class: "text-xl", "Children:" }
        ul {
          // TODO get note info and use title and link.
          for note in children {
            li {
              a { "{note.0.to_string()}" }
            }
          }
        }
      }
      if !notes_this_blocks.is_empty() {
        br {}
        p { class: "text-xl", "Notes blocked by this:" }
        ul {
          // TODO get note info and use title and link.
          for note in notes_this_blocks {
            li {
              a { "{note.0.to_string()}" }
            }
          }
        }
      }
      if !notes_blocking_this.is_empty() {
        br {}
        p { class: "text-xl", "Notes blocking this:" }
        ul {
          // TODO get note info and use title and link.
          for note in notes_blocking_this {
            li {
              a { "{note.0.to_string()}" }
            }
          }
        }
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
