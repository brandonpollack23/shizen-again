//! The core shizen shared UI.
//!
//! The actual shizen database and other platform specific functionality are provided by each
//! individual app (Desktop etc).
#![allow(non_snake_case)]

use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
  #[route("/")]
  NoteListView { show_blocked: bool },
}

#[component]
pub fn App() -> Element {
  rsx! {
      Router::<Route> {}
  }
}

#[component]
fn NoteListView(show_blocked: bool) -> Element {
  rsx! {
    "Hello Shizen"
  }
}
