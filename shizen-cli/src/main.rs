use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::{ArgAction, Parser, Subcommand};
use libshizen::entities::{Note, NoteId, PeerId, PeerInfo};
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
  Complete {
    note_id: Uuid,
    #[arg(action = ArgAction::Set)]
    completed: bool,
  },
  #[command(visible_alias = "ls")]
  List {
    #[arg(short, long, default_value_t = false)]
    show_blocked: bool,
  },
  #[command(visible_alias = "a")]
  Add {
    title: String,
    #[arg(short, long)]
    description: Option<String>,
    #[arg(short, long)]
    parent_id: Option<String>,
  },
  #[command(visible_alias = "rm")]
  Remove {
    /// Parsable UUID of parent note.
    uuid: String,
  },
  #[command(visible_alias = "up")]
  Update {
    #[arg(short, long)]
    note_id: String,
    #[arg(short, long)]
    title: Option<String>,
    #[arg(short, long)]
    description: Option<String>,
    #[arg(short, long, default_value_t = false)]
    remove_description: bool,
  },
  #[command(visible_alias = "dep")]
  AddDependency {
    from: String,
    to: String,
  },
  Reorder {
    #[command(subcommand)]
    command: ReorderCommand,
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
  Peer {
    #[command(subcommand)]
    command: PeerCommand,
  },
  /// Sync with all peers or a specified one.
  Sync {
    #[arg(short, long)]
    peer: Option<Uuid>,
  },
}

#[derive(Subcommand, PartialEq, Eq)]
enum ReorderCommand {
  #[command(visible_alias = "b")]
  Before { id: NoteId, before: NoteId },
  #[command(visible_alias = "a")]
  After { id: NoteId, after: NoteId },
}

#[derive(Subcommand, PartialEq, Eq)]
enum PeerCommand {
  #[command(visible_alias = "ls")]
  List,
  #[command(visible_alias = "a")]
  Add {
    remote_address: SocketAddr,
    local_server_port: Option<u16>,
  },
  #[command(visible_alias = "rm")]
  Remove { peer_id: Uuid },
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

  let db = database.unwrap();

  match cli.command {
    Commands::Create => unreachable!("This case is handled explicitly above"),
    Commands::Complete { note_id, completed } => db
      .set_completed(&NoteId(note_id), completed)
      .expect("error setting note completion"),
    Commands::List { show_blocked } => {
      if show_blocked {
        println!(
          "{}",
          format_note_list(&db.load_all_notes().expect("error loading all notes"))
        );
      } else {
        println!("Warning, hiding blocked items, show them with \"-s\"\n");
        println!(
          "{}",
          format_note_list(
            &db
              .load_all_unblocked_notes(false)
              .expect("error loading all notes")
          )
        );
      }
    }
    Commands::Add {
      title,
      description,
      parent_id,
    } => {
      let parent_id = parent_id
        .map(|p| Uuid::parse_str(&p).expect("invalid parent uuid"))
        .map(NoteId);
      let added_note = db
        .create_new_note(&title, description.as_deref(), parent_id.as_ref())
        .expect("could not add note");
      println!("Note added:\n\n{:#?}", added_note);
    }
    Commands::Remove { uuid } => {
      let uuid = Uuid::parse_str(&uuid).expect("Could not parse UUID");
      db.delete_note(&NoteId(uuid))
        .expect("Could not delete note");
    }
    Commands::Update {
      note_id,
      title,
      description,
      remove_description,
    } => {
      let note_id = NoteId(Uuid::parse_str(&note_id).expect("could not parse note id"));
      if let Some(t) = title {
        db.update_title(&note_id, &t).expect("error updating title");
      }

      if remove_description {
        db.update_description(&note_id, None)
          .expect("Error removing description");
      } else if let Some(d) = description {
        db.update_description(&note_id, Some(&d))
          .expect("error updating description");
      }
    }
    Commands::AddDependency { from, to } => {
      let from = NoteId(Uuid::parse_str(&from).expect("could not parse note id"));
      let to = NoteId(Uuid::parse_str(&to).expect("could not parse note id"));
      db.add_blocked_note(&from, &to)
        .expect("error adding dependency");
    }
    Commands::Undo => {
      db.undo().expect("Failed to undo");
    }
    Commands::Redo => {
      db.redo().expect("Failed to redo");
    }
    Commands::History => {
      let muts = db
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
      let muts = db
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
      let database_id = db.get_peer_id().unwrap().0.to_string();
      println!("Shizen server with database id {database_id} listening on port {port}");
      let mut server =
        libshizen::SyncServer::listen_on_thread(("localhost", port), cli.database_path.into())
          .expect("Could not create server");
      server.join();
    }
    Commands::Peer { command } => {
      handle_peer_command(command, &db);
    }
    Commands::Sync { peer } => {
      if let Some(p) = peer {
        let peer_info = db.get_peer(&PeerId(p)).expect("No such peer");
        db.sync_with_peer(&peer_info)
          .expect("Error syncing with peer");
      } else {
        let peers = db.get_peers().expect("Could not get peers");
        for p in &peers {
          db.sync_with_peer(p).expect("Error syncing with peer");
        }
      }
    }
    Commands::Reorder { command } => {
      handle_reorder_command(command, &db);
    }
  }
}

fn handle_reorder_command(command: ReorderCommand, db: &RusqliteStorage) {
  match command {
    ReorderCommand::Before { id, before } => db
      .adjust_rank_between(&id, Some(&before), None)
      .expect("Could not adjust note position"),
    ReorderCommand::After { id, after } => db
      .adjust_rank_between(&id, None, Some(&after))
      .expect("Could not adjust note position"),
  }
}

fn handle_peer_command(command: PeerCommand, db: &RusqliteStorage) {
  match command {
    PeerCommand::List => {
      let peers = db.get_peers().expect("Could not load peers");
      for peer in &peers {
        println!("{}", format_peer(peer));
      }
    }
    PeerCommand::Add {
      remote_address,
      local_server_port,
    } => {
      let local_server_addr =
        local_server_port.map(|p| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), p));
      db.add_peer(&remote_address, local_server_addr.as_ref())
        .expect("Error adding peer");
    }
    PeerCommand::Remove { peer_id } => db
      .remove_peer(&PeerId(peer_id))
      .expect("Could not remove peer"),
  }
}

fn format_peer(peer: &PeerInfo) -> String {
  format!("Peer Id: {} Address: {}", peer.peer_id.0, peer.addr)
}

fn format_note(note: &Note) -> String {
  format!(
    "{}{} -- ({})\n  {}\n  Parent: {}\n  Blocking: {}\n  Blocked By: {}",
    if note.completed { ":DONE: " } else { "" },
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
      .map(|n| format!("{}", n))
      .collect::<Vec<_>>()
      .join(","),
    note
      .notes_blocking_this
      .iter()
      .map(|n| format!("{}", n))
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
