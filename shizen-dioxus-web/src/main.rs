#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};
use libshizen::{storage::TodoStorage, DefaultStorage};
use shizen_dioxus_core::App;

fn main() {
  // Init logger
  dioxus_logger::init(Level::INFO).expect("failed to init logger");
  launch(WebApp);
}

fn WebApp() -> Element {
  use_context_provider(|| {
    info!("Web only supports in memory db for now");
    let db: Box<dyn TodoStorage> = Box::new(DefaultStorage::open(None).expect("could not open db"));
    return Signal::new(db);
  });

  rsx! {
    App {}
  }
}
