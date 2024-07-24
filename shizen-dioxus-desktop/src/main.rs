#![allow(non_snake_case)]

use dioxus::prelude::*;
use shizen_dioxus_core::*;
use tracing::Level;

fn main() {
  // Init logger
  dioxus_logger::init(Level::INFO).expect("failed to init logger");

  let cfg = dioxus::desktop::Config::new()
    .with_custom_head(r#"<link rel="stylesheet" href="tailwind.css">"#.to_string());
  LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}
