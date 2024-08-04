#![allow(non_snake_case)]

use std::path::PathBuf;

use dioxus::prelude::*;
use libshizen::{storage::TodoStorage, DefaultStorage};
use shizen_dioxus_core::App;
use tracing::{info, Level};

fn main() {
  // TODO configuration with logging level.

  // Init logger
  dioxus_logger::init(Level::INFO).expect("failed to init logger");

  let cfg = dioxus::desktop::Config::new()
    .with_custom_head(r#"<link rel="stylesheet" href="tailwind.css">"#.to_string());
  LaunchBuilder::desktop().with_cfg(cfg).launch(DesktopApp);
}

fn DesktopApp() -> Element {
  use_context_provider(|| {
    // TODO have settings object that contains selected database path.
    // let default_storage_path = format!("{}/shizen.db", env!("HOME"));
    let default_storage_path: PathBuf = "testdb/test.db".into();

    info!("Attempting to create database at path {default_storage_path:?}");

    // TODO combine open with create with a flag for creation param.
    std::fs::create_dir_all(&default_storage_path.parent().unwrap()).unwrap();
    if let Err(_) = libshizen::DefaultStorage::create(&default_storage_path) {
      info!("Database already exists");
    } else {
      info!("New Database created");
    }

    let db: Box<dyn TodoStorage> = Box::new(
      DefaultStorage::open(Some(&default_storage_path.into())).expect("could not open db"),
    );

    return Signal::new(db);
  });

  rsx! {
    App {}
  }
}
