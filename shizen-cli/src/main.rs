use clap::{Parser, Subcommand};
use libshizen::storage::rusqlite::RusqliteStorage;
use libshizen::storage::TodoStorage;
use libshizen::ShizenError;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[arg(short, long, value_name = "DB FILE", default_value_t = format!("{}/shizen.db", env!("HOME")))]
  database_path: String,

  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand, PartialEq, Eq)]
#[command(arg_required_else_help(true))]
enum Commands {
  /// List notes/todos.
  Create,
  List,
}

fn main() {
  tracing_subscriber::fmt::init();

  let cli = Cli::parse();

  if cli.command == Commands::Create {
    if let Err(e) = RusqliteStorage::create(&cli.database_path.clone().into()) {
      eprintln!("Error creating database: {}", e.to_string());
      std::process::exit(1);
    }
    std::process::exit(0);
  }

  let database = RusqliteStorage::open(Some(&cli.database_path.clone().into()));
  if let Err(ShizenError::ErrorOpeningDb(_)) = database {
    eprintln!(
      "Error opening database file: {}\nAre you sure it exists? Create with the create subcommand",
      cli.database_path
    );
    std::process::exit(1)
  }

  let database = database.unwrap();

  match cli.command {
    Commands::List => {
      // TODO prettier
      println!(
        "{:#?}",
        database.load_all_notes().expect("error loading all notes")
      );
    }
    Commands::Create => unreachable!("This case is handled explicitly above"),
  }
}
