#![allow(non_snake_case)]

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
  info!("starting app");

  let cfg = dioxus::desktop::Config::new()
    .with_custom_head(r#"<link rel="stylesheet" href="tailwind.css">"#.to_string());
  LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

#[component]
fn App() -> Element {
  rsx! {
      Router::<Route> {}
  }
}

#[component]
fn NoteListView() -> Element {
  // TODO show_blocked

  let db = use_context::<Signal<Box<dyn TodoStorage>>>();
  // TODO async load all unblocked notes in a coroutine or something.

  rsx! {
    if let Ok(unblocked_notes) = db.read().load_all_unblocked_notes() {
      for note in unblocked_notes {
        TodoListItem { note }
      }
    } else {
      div { class: "text-red", "Error Loading Notes!" }
    }
  }
}

// TODO toggle to show things blocking this note.
#[component]
fn TodoListItem(note: Note) -> Element {
  let blocklist_str = note
    .notes_this_blocks
    .iter()
    .map(|n| n.0.to_string())
    .collect::<Vec<_>>()
    .join(", ");

  rsx! {
    // TODO parent sorting.
    div {
      div { class: "font-semibold", "{note.title}" }
      if note.description.is_some() {
        div { class: "italic", "{note.description.unwrap()}" }
      }
      if note.notes_this_blocks.len() > 0 {
        div { class: "italic",
          span { "Blocking: [" } span { class: "text-ellipsis", "Blocks: {blocklist_str}" } span { "]" }
        }
      }
      // TODO blocked by with toggle.
    }
  }
}
