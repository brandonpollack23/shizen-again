use std::{
  io::{Read, Write},
  net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
  sync::mpsc::Sender,
  thread::{self, JoinHandle},
  time::Duration,
};

use serde::{Deserialize, Serialize};
use tracing::{error, info, trace};

use crate::{
  entities::{Action, PeerId, PeerInfo},
  storage::{rusqlite::RusqliteStorage, TodoStorage},
  ShizenError, ShizenResult,
};

pub struct SyncConnection {
  this_peer_id: PeerId,
  this_addr: Option<SocketAddr>,
  stream: TcpStream,
}

impl SyncConnection {
  pub fn new<A: ToSocketAddrs>(
    this_peer_id: &PeerId,
    addr: A,
    // TODO NOW change to just a port and let the ip come from the request.
    this_addr: Option<A>,
  ) -> ShizenResult<SyncConnection> {
    let this_peer_id = this_peer_id.clone();
    let this_addr = this_addr.map(|a| a.to_socket_addrs().unwrap().next().unwrap());
    let stream = TcpStream::connect(addr)?;
    Ok(SyncConnection {
      this_peer_id,
      this_addr,
      stream,
    })
  }

  pub fn peer_id_handshake(&mut self) -> ShizenResult<PeerId> {
    let response = sync_protocol_tx(
      &mut self.stream,
      &SyncRequest::PeerIdentificationHandshake {
        local_peer_id: self.this_peer_id.clone(),
        local_server_address: self
          .this_addr
          .map(|a| a.to_socket_addrs().unwrap().next().unwrap()),
      },
    )?;

    match response {
      SyncResponse::PeerIdentificationHandshakeResponse(r) => Ok(r),
      other => Err(ShizenError::UnexpectedSyncProtocolResponse(
        "PeerIdentificationHandshakeResponse".to_string(),
        other,
      )),
    }
  }

  pub fn sync_with_peer(
    &mut self,
    database: &RusqliteStorage,
    peer: &PeerInfo,
  ) -> ShizenResult<SyncResults> {
    // 2. Request all changes since last synced change
    let response = sync_protocol_tx(
      &mut self.stream,
      &SyncRequest::Sync {
        last_sync_clock: peer.clock,
        local_peer_id: self.this_peer_id.clone(),
      },
    )?;

    let (clock, changes) = match response {
      SyncResponse::SyncResponse { clock, changes } => (clock, changes),
      other => {
        return Err(ShizenError::UnexpectedSyncProtocolResponse(
          "SyncResponse".to_string(),
          other,
        ));
      }
    };

    info!(
      "Received sync response with {} changes and clock of {}",
      changes.len(),
      clock
    );
    trace!("Changes received: {changes:#?}");

    // 3. Rebase our changes on top of these and increment our version to match.
    // First undo all our local changes
    let mut changes_undone = 0usize;
    loop {
      let clock = database.undo()?;
      changes_undone = changes_undone + 1;

      if clock == 0 || clock <= peer.clock {
        break;
      }
    }

    // Insert each of these in the database
    for change in &changes {
      database.apply_action(&change)?;
    }

    // Replace all our changes.
    while changes_undone > 0 {
      // TODO handle merge conflicts (attempt to merge when same notes modified).
      database.redo()?;
      changes_undone = changes_undone - 1;
    }

    // 4. Update the synced version of this peer in the peers table
    database.set_peer_clock(&peer.peer_id, clock)?;

    // 5. TODO in the recieving in notify of some way to request sync back.

    info!("Sync completed");
    Ok(SyncResults {
      updated_peer_clock: clock,
      num_changes: changes.len(),
    })
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
  PeerIdentificationHandshake {
    local_peer_id: PeerId,
    local_server_address: Option<SocketAddr>,
  },
  Sync {
    local_peer_id: PeerId,
    last_sync_clock: usize,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
  PeerIdentificationHandshakeResponse(PeerId),
  SyncResponse { clock: usize, changes: Vec<Action> },
}

pub struct SyncResults {
  updated_peer_clock: usize,
  num_changes: usize,
}

// TODO spawn threads to sync multiple peers at once?

/// Server that will listen on sync connections (blocking) as long as it is constructed.
pub struct SyncServer {
  thread: Option<JoinHandle<()>>,
  kill_tx: Sender<()>,
}

impl SyncServer {
  pub fn listen_on_thread<A: ToSocketAddrs>(
    addr: A,
    database_path: std::path::PathBuf,
  ) -> ShizenResult<SyncServer> {
    let server = TcpListener::bind(addr)?;
    server.set_nonblocking(true)?;
    let (kill_tx, kill_rx) = std::sync::mpsc::channel::<()>();

    let thread = Some(thread::spawn(move || {
      let database = RusqliteStorage::open(Some(&database_path)).unwrap();
      let this_peer_id = database.get_peer_id().unwrap();

      loop {
        if let Ok(()) = kill_rx.try_recv() {
          break;
        }

        match server.accept() {
          Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // Wait a bit before trying again
            thread::sleep(Duration::from_millis(100));
            continue;
          }
          Err(e) => error!("Tcp accept error occurred: {e:#?}"),
          Ok((socket, _addr)) => {
            if let Err(e) = Self::handle_request(socket, &this_peer_id, &database) {
              error!("Error handling connection: {e:#?}");
            }
          }
        }
      }
    }));

    Ok(SyncServer { thread, kill_tx })
  }

  fn handle_request(
    mut stream: TcpStream,
    this_peer_id: &PeerId,
    database: &RusqliteStorage,
  ) -> ShizenResult<()> {
    sync_protocol_rx(&mut stream, this_peer_id, database)
  }
}

impl Drop for SyncServer {
  fn drop(&mut self) {
    if let Err(e) = self.kill_tx.send(()) {
      error!("Error when killing sync server: {e:#?}");
    }

    if let Err(e) = self.thread.take().unwrap().join() {
      error!("Error when joining sync server thread: {e:#?}");
    }
  }
}

fn sync_protocol_tx(
  stream: &mut TcpStream,
  sync_message: &SyncRequest,
) -> Result<SyncResponse, ShizenError> {
  trace!("Sending message {sync_message:#?} to peer");

  serialize_message_to_stream(sync_message, stream)?;

  let mut response_len_bytes = [0; 4];
  stream.read_exact(&mut response_len_bytes)?;

  let response_len = u32::from_be_bytes(response_len_bytes) as usize;
  let mut response_bytes = vec![0; response_len];
  stream.read_exact(&mut response_bytes)?;

  let response: SyncResponse = serde_json::from_slice(&response_bytes)?;
  Ok(response)
}

fn serialize_message_to_stream<M: Serialize>(
  message: &M,
  stream: &mut TcpStream,
) -> Result<(), ShizenError> {
  let message = serde_json::to_string(message)?;
  let len_bytes: u32 = message.len() as u32;
  stream.write(&len_bytes.to_be_bytes())?;
  stream.write(message.as_bytes())?;
  Ok(())
}

fn sync_protocol_rx(
  stream: &mut TcpStream,
  this_peer_id: &PeerId,
  database: &RusqliteStorage,
) -> ShizenResult<()> {
  let mut request_len_bytes = [0; 4];
  stream.read_exact(&mut request_len_bytes)?;
  let request_len = u32::from_be_bytes(request_len_bytes) as usize;
  let mut request_bytes = vec![0; request_len];
  stream.read_exact(&mut request_bytes)?;

  let request: SyncRequest = serde_json::from_slice(&request_bytes)?;

  match request {
    SyncRequest::PeerIdentificationHandshake {
      local_peer_id: other_peer_id,
      local_server_address: other_server_address,
    } => {
      let other_clock = 0; // No syncing done yet, always start at 0.
      if let Some(a) = other_server_address {
        database.add_connected_peer(other_peer_id, other_clock, &a)?;
      }

      serialize_message_to_stream(
        &SyncResponse::PeerIdentificationHandshakeResponse(this_peer_id.clone()),
        stream,
      )?;
    }
    SyncRequest::Sync {
      last_sync_clock,
      local_peer_id: other_peer_id,
    } => {
      let changes = database.load_all_changes_since_clock(last_sync_clock)?;
      let clock = database.get_clock()?;

      // The peer is at least synced up to our clock now.
      database.set_peer_clock(&other_peer_id, clock)?;

      serialize_message_to_stream(&SyncResponse::SyncResponse { changes, clock }, stream)?;
    }
  }

  Ok(())
}

#[cfg(test)]
mod test {
  use std::net::ToSocketAddrs;

  use tracing::info;
  use tracing_test::traced_test;

  use crate::storage::{rusqlite::RusqliteStorage, TodoStorage};

  use super::SyncServer;

  #[test]
  #[traced_test]
  fn peer_handshake() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file:peer_handshake?mode=memory&cache=shared";
    let in_memory_uri2 = "file:peer_handshake2?mode=memory&cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let first_pid = first.get_peer_id().unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1701", in_memory_uri.into()).unwrap();
    let _second_syncer =
      SyncServer::listen_on_thread("localhost:1801", in_memory_uri2.into()).unwrap();

    let second = RusqliteStorage::open(None).unwrap();

    let second_pid = second.get_peer_id().unwrap();

    let expected_first_pid = second
      .add_peer("localhost:1701", Some("localhost:1801"))
      .unwrap();
    assert_eq!(expected_first_pid, first_pid);

    let seconds_peers = second.get_peers().unwrap();
    assert_eq!(seconds_peers.len(), 1);
    assert_eq!(seconds_peers[0].peer_id, first_pid);
    assert_eq!(seconds_peers[0].clock, 0);
    assert_eq!(
      seconds_peers[0].addr,
      "localhost:1701".to_socket_addrs().unwrap().next().unwrap()
    );

    let peers = &first.get_peers().unwrap();
    info!("peers {:#?}", peers);

    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].peer_id, second_pid);
    assert_eq!(peers[0].clock, 0);
  }

  #[test]
  #[traced_test]
  fn peer_sync_one_way() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file:peer_sync_one_way?mode=memory&cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1702", in_memory_uri.into()).unwrap();

    let second = RusqliteStorage::open(None).unwrap();
    let sync_peer_id = second.add_peer("localhost:1702", None).unwrap();
    let peer = second.get_peer(&sync_peer_id).unwrap();

    let picard = first
      .create_new_note("Picard", Some("Captain"), None)
      .unwrap();
    let riker = first
      .create_new_note("Riker", Some("Number One"), Some(&picard.id))
      .unwrap();
    let picard = first.load_note(&picard.id).unwrap();

    let sync_results = second.sync_with_peer(&peer).unwrap();
    assert_eq!(sync_results.num_changes, 2);
    assert_eq!(sync_results.updated_peer_clock, 2);

    let synced_notes = second.load_all_notes().unwrap();
    assert_eq!(synced_notes.len(), 2);

    let picard_synced = synced_notes.iter().find(|n| n.title == "Picard").unwrap();
    let riker_synced = synced_notes.iter().find(|n| n.title == "Riker").unwrap();
    assert_eq!(picard, picard_synced.clone());
    assert_eq!(riker, riker_synced.clone());
  }

  #[test]
  #[traced_test]
  fn peer_sync_local_added_notes_one_way() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file:peer_sync_local_added_notes_one_way?mode=memory&cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1703", in_memory_uri.into()).unwrap();

    let second = RusqliteStorage::open(None).unwrap();
    let sync_peer_id = second.add_peer("localhost:1703", None).unwrap();
    let peer = second.get_peer(&sync_peer_id).unwrap();

    let picard = first
      .create_new_note("Picard", Some("Captain"), None)
      .unwrap();
    let riker = first
      .create_new_note("Riker", Some("Number One"), Some(&picard.id))
      .unwrap();
    let picard = first.load_note(&picard.id).unwrap();

    let locally_added_bev = second.create_new_note("Bev", Some("Doc"), None).unwrap();
    let locally_added_wes = second
      .create_new_note("Wes", Some("Bearded"), None)
      .unwrap();

    let sync_results = second.sync_with_peer(&peer).unwrap();
    assert_eq!(sync_results.num_changes, 2);
    assert_eq!(sync_results.updated_peer_clock, 2);

    let all_notes_second = second.load_all_notes().unwrap();
    let picard_synced = all_notes_second
      .iter()
      .find(|n| n.title == "Picard")
      .unwrap();
    let riker_synced = all_notes_second
      .iter()
      .find(|n| n.title == "Riker")
      .unwrap();
    let bev_after_sync = all_notes_second.iter().find(|n| n.title == "Bev").unwrap();
    let wes_after_sync = all_notes_second.iter().find(|n| n.title == "Wes").unwrap();
    assert_eq!(all_notes_second.len(), 4);

    assert_eq!(picard, picard_synced.clone());
    assert_eq!(riker, riker_synced.clone());
    assert_eq!(locally_added_bev, bev_after_sync.clone());
    assert_eq!(locally_added_wes, wes_after_sync.clone());
  }

  #[test]
  #[traced_test]
  fn local_title_update_sync() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file:local_title_update_sync?mode=memory&cache=shared";
    let in_memory_uri2 = "file:local_title_update_sync_second?mode=memory&cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1704", in_memory_uri.into()).unwrap();
    let second = RusqliteStorage::open(Some(&in_memory_uri2.into())).unwrap();
    let _second_syncer =
      SyncServer::listen_on_thread("localhost:1804", in_memory_uri2.into()).unwrap();

    let sync_peer_id = second
      .add_peer("localhost:1704", Some("localhost:1804"))
      .unwrap();
    let peer = second.get_peer(&sync_peer_id).unwrap();

    let picard = first
      .create_new_note("Picard", Some("Captain"), None)
      .unwrap();
    let _riker = first
      .create_new_note("Riker", Some("Number One"), Some(&picard.id))
      .unwrap();
    let picard = first.load_note(&picard.id).unwrap();

    let sync_results = second.sync_with_peer(&peer).unwrap();
    assert_eq!(sync_results.num_changes, 2);
    assert_eq!(sync_results.updated_peer_clock, 2);

    // first.update_title(&picard.id, "Positronic").unwrap();
    second.update_title(&picard.id, "Locutus").unwrap();
    let second_clock = second.get_clock().unwrap();
    assert_eq!(second_clock, 3);
    let changes_that_would_sync = second.load_all_changes_since_clock(2).unwrap();
    assert_eq!(changes_that_would_sync.len(), 1);

    let second_peer_info = first.get_peer(&second.get_peer_id().unwrap()).unwrap();
    let backsync_results = first.sync_with_peer(&second_peer_info).unwrap();
    assert_eq!(backsync_results.num_changes, 1);
    assert_eq!(sync_results.updated_peer_clock, 2);
  }

  #[test]
  #[traced_test]
  fn local_desc_update_sync() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file:local_desc_update_sync?mode=memory&cache=shared";
    let in_memory_uri2 = "file:local_desc_update_sync2?mode=memory&cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1705", in_memory_uri.into()).unwrap();

    let second = RusqliteStorage::open(Some(&in_memory_uri2.into())).unwrap();
    let _second_syncer =
      SyncServer::listen_on_thread("localhost:1805", in_memory_uri2.into()).unwrap();
    let first_sync_peer_id = second
      .add_peer("localhost:1705", Some("localhost:1805"))
      .unwrap();
    let first_peer = second.get_peer(&first_sync_peer_id).unwrap();

    let picard = first
      .create_new_note("Picard", Some("Captain"), None)
      .unwrap();
    let _riker = first
      .create_new_note("Riker", Some("Number One"), Some(&picard.id))
      .unwrap();
    let picard = first.load_note(&picard.id).unwrap();

    let sync_results = second.sync_with_peer(&first_peer).unwrap();
    assert_eq!(sync_results.num_changes, 2);
    assert_eq!(sync_results.updated_peer_clock, 2);

    second
      .update_description(&picard.id, Some("of borg"))
      .unwrap();
    let second_clock = second.get_clock().unwrap();
    assert_eq!(second_clock, 3);
    let changes_that_would_sync = second.load_all_changes_since_clock(2).unwrap();
    assert_eq!(changes_that_would_sync.len(), 1);

    let second_peer_info = first.get_peer(&second.get_peer_id().unwrap()).unwrap();
    let backsync_results = first.sync_with_peer(&second_peer_info).unwrap();
    assert_eq!(backsync_results.num_changes, 1);
    assert_eq!(sync_results.updated_peer_clock, 2);
  }

  // TODO when merge conflicts can be detected this test should do so.
  #[test]
  #[traced_test]
  fn local_and_remote_keeps_local() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file:local_and_remote_keeps_local?mode=memory&cache=shared";
    let in_memory_uri2 = "file:local_and_remote_keeps_local_second?mode=memory&cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1704", in_memory_uri.into()).unwrap();
    let second = RusqliteStorage::open(Some(&in_memory_uri2.into())).unwrap();
    let _second_syncer =
      SyncServer::listen_on_thread("localhost:1804", in_memory_uri2.into()).unwrap();

    let sync_peer_id = second
      .add_peer("localhost:1704", Some("localhost:1804"))
      .unwrap();
    let peer = second.get_peer(&sync_peer_id).unwrap();

    let picard = first
      .create_new_note("Picard", Some("Captain"), None)
      .unwrap();
    let _riker = first
      .create_new_note("Riker", Some("Number One"), Some(&picard.id))
      .unwrap();
    let picard = first.load_note(&picard.id).unwrap();

    let sync_results = second.sync_with_peer(&peer).unwrap();
    assert_eq!(sync_results.num_changes, 2);
    assert_eq!(sync_results.updated_peer_clock, 2);

    second.update_title(&picard.id, "Locutus").unwrap();
    let second_clock = second.get_clock().unwrap();
    assert_eq!(second_clock, 3);
    let changes_that_would_sync = second.load_all_changes_since_clock(2).unwrap();
    assert_eq!(changes_that_would_sync.len(), 1);

    first.update_title(&picard.id, "Positronic").unwrap();

    let second_peer_info = first.get_peer(&second.get_peer_id().unwrap()).unwrap();
    let _ = first.sync_with_peer(&second_peer_info).unwrap();

    let picard_readback = first.load_note(&picard.id).unwrap();
    assert_eq!(picard_readback.title, "Positronic");

    let mutations = first.load_all_changes_since_clock(0).unwrap();
    info!("Mutations: {mutations:#?}");

    // Undoing should get the old title from remote in the history.
    first.undo().unwrap();
    let picard_readback_after_undo = first.load_note(&picard.id).unwrap();
    assert_eq!(picard_readback_after_undo.title, "Picard");
  }

  // TODO do sync with all updates done locally and remotely.
}
