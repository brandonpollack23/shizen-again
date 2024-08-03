use clap::{Args, Parser, Subcommand};
use libshizen::entities::{Note, NoteId};
use libshizen::storage::rusqlite::RusqliteStorage;
use libshizen::storage::TodoStorage;
use libshizen::ShizenError;
use uuid::Uuid;

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
  #[command(alias = "cr")]
  Create,
  #[command(alias = "ls")]
  List,
  #[command(alias = "a")]
  Add(AddArguments),
  #[command(alias = "rm")]
  Remove {
    /// Parsable UUID of parent note.
    uuid: String,
  },
  // TODO update title/desc/parent notes
  // TODO update dependencies
}

#[derive(Args, Debug, PartialEq, Eq)]
struct AddArguments {
  title: String,
  #[arg(short, long)]
  description: Option<String>,
  #[arg(short, long)]
  parent_id: Option<String>,
}

fn main() {
  tracing_subscriber::fmt::init();

  let cli = Cli::parse();

  if cli.command == Commands::Create {
    if let Err(e) = RusqliteStorage::create(&cli.database_path.clone().into()) {
      eprintln!("Error creating database: {:?}", e);
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

  let mut database = database.unwrap();

  match cli.command {
    Commands::List => {
      // TODO prettier
      println!(
        "{}",
        format_note_list(&database.load_all_notes().expect("error loading all notes"))
      );
    }
    Commands::Create => unreachable!("This case is handled explicitly above"),
    Commands::Add(args) => {
      let parent_id = args
        .parent_id
        .map(|p| Uuid::parse_str(&p).expect("invalid parent uuid"))
        .map(NoteId);
      let added_note = database
        .create_new_note(&args.title, args.description.as_deref(), parent_id.as_ref())
        .expect("could not add note");
      println!("Note added:\n\n{:#?}", added_note);
    }
    Commands::Remove { uuid } => {
      let uuid = Uuid::parse_str(&uuid).expect("Could not parse UUID");
      database
        .delete_note(&NoteId(uuid))
        .expect("Could not delete note");
    }
  }
}

fn format_note(note: &Note) -> String {
  format!(
    "{} -- ({})\n  {}",
    note.title,
    note.id,
    note.description.clone().unwrap_or("---".to_string())
  )
}

fn format_note_list(notes: &[Note]) -> String {
  let mut list = String::new();
  for n in notes {
    list.push_str(&format!("• {}\n\n", format_note(n)))
  }

  list
}
