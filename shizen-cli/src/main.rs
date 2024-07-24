use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
  ListAllNotes,
  ListAllActionableNotes,
  // etc
}

fn main() {
  // TODO CLI set up tracing_subscriber.
  println!("Hello, world!");
}
