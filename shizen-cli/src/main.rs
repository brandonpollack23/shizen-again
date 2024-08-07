use clap::{Args, Parser, Subcommand};
use libshizen::entities::{Note, NoteId};
use libshizen::storage::rusqlite::RusqliteStorage;
use libshizen::storage::TodoStorage;
use libshizen::ShizenError;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
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
  #[command(visible_alias = "cr")]
  Create,
  #[command(visible_alias = "ls")]
  List {
    #[arg(short, long, default_value_t = false)]
    show_blocked: bool,
  },
  #[command(visible_alias = "a")]
  Add(AddArguments),
  #[command(visible_alias = "rm")]
  Remove {
    /// Parsable UUID of parent note.
    uuid: String,
  },
  #[command(visible_alias = "up")]
  Update(UpdateArguments),
  #[command(visible_alias = "dep")]
  AddDependency {
    from: String,
    to: String,
  },
  Undo,
  Redo,
  #[command(visible_alias = "h")]
  History,
  RedoQueue,
  Serve {
    #[arg(short, long, default_value_t = 1701u16)]
    port: u16,
  },
}

#[derive(Args, Debug, PartialEq, Eq)]
struct AddArguments {
  title: String,
  #[arg(short, long)]
  description: Option<String>,
  #[arg(short, long)]
  parent_id: Option<String>,
}

#[derive(Args, Debug, PartialEq, Eq)]
struct UpdateArguments {
  #[arg(short, long)]
  note_id: String,
  #[arg(short, long)]
  title: Option<String>,
  #[arg(short, long)]
  description: Option<String>,
  #[arg(short, long, default_value_t = false)]
  remove_description: bool,
}

fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(
      EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .from_env_lossy(),
    )
    .init();

  let cli = Cli::parse();

  if cli.command == Commands::Create {
    if let Err(e) = RusqliteStorage::open_create(Some(&cli.database_path.clone().into()), true) {
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

  let database = database.unwrap();

  match cli.command {
    Commands::Create => unreachable!("This case is handled explicitly above"),
    Commands::List { show_blocked } => {
      if show_blocked {
        println!(
          "{}",
          format_note_list(&database.load_all_notes().expect("error loading all notes"))
        );
      } else {
        println!("Warning, hiding blocked items, show them with \"-s\"\n");
        println!(
          "{}",
          format_note_list(
            &database
              .load_all_unblocked_notes()
              .expect("error loading all notes")
          )
        );
      }
    }
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
    Commands::Update(args) => {
      let note_id = NoteId(Uuid::parse_str(&args.note_id).expect("could not parse note id"));
      if let Some(t) = args.title {
        database
          .update_title(&note_id, &t)
          .expect("error updating title");
      }

      if args.remove_description {
        database
          .update_description(&note_id, None)
          .expect("Error removing description");
      } else if let Some(d) = args.description {
        database
          .update_description(&note_id, Some(&d))
          .expect("error updating description");
      }
    }
    Commands::AddDependency { from, to } => {
      let from = NoteId(Uuid::parse_str(&from).expect("could not parse note id"));
      let to = NoteId(Uuid::parse_str(&to).expect("could not parse note id"));
      database
        .add_blocked_note(&from, &to)
        .expect("error adding dependency");
    }
    Commands::Undo => {
      database.undo().expect("Failed to undo");
    }
    Commands::Redo => {
      database.redo().expect("Failed to redo");
    }
    Commands::History => {
      let muts = database
        .load_all_changes_since_clock(0)
        .expect("Could not load mutations");

      for m in muts {
        println!(
          "{}",
          serde_json::to_string_pretty(&m).expect("Error formatting json")
        );
      }
    }
    Commands::RedoQueue => {
      let muts = database
        .load_redo_queue()
        .expect("Could not load redo queue mutations");

      for m in muts {
        println!(
          "{}",
          serde_json::to_string_pretty(&m).expect("Error formatting json")
        );
      }
    }
    Commands::Serve { port } => {
      let mut server =
        libshizen::SyncServer::listen_on_thread(("localhost", port), cli.database_path.into())
          .expect("Could not create server");
      server.join();
    }
  }
}

fn format_note(note: &Note) -> String {
  format!(
    "{} -- ({})\n  {}\n  Parent: {}\n  Blocking: {}\n  Blocked By: {}",
    note.title,
    note.id,
    note.description.clone().unwrap_or("---".to_string()),
    note
      .parent_id
      .as_ref()
      .map(|p| p.0.to_string())
      .unwrap_or("None".to_string()),
    note
      .notes_this_blocks
      .iter()
      .map(|n| format!("{}", n.to_string()))
      .collect::<Vec<_>>()
      .join(","),
    note
      .notes_blocking_this
      .iter()
      .map(|n| format!("{}", n.to_string()))
      .collect::<Vec<_>>()
      .join(","),
  )
}

fn format_note_list(notes: &[Note]) -> String {
  let mut list = String::new();
  for n in notes {
    list.push_str(&format!("• {}\n\n", format_note(n)))
  }

  list
}
