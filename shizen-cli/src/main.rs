use clap::{Parser, Subcommand};
use libshizen::storage::rusqlite::RusqliteStorage;
use libshizen::storage::TodoStorage;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[arg(short, long, value_name = "DB FILE", default_value_t = ("~/shizen.db".to_owned()))]
  database_path: String,

  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
#[command(arg_required_else_help(true))]
enum Commands {
  /// List notes/todos.
  List,
}

fn main() {
  tracing_subscriber::fmt::init();

  let cli = Cli::parse();

  let mut database =
    RusqliteStorage::new(Some(cli.database_path.into())).expect("Could not open database");

  match cli.command {
    Commands::List => {
      // TODO prettier
      println!(
        "{:#?}",
        database.load_all_notes().expect("error loading all notes")
      );
    }
  }
}
