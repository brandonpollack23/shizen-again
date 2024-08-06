use std::{
  io::{Read, Write},
  net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
  sync::mpsc::Sender,
  thread::{self, JoinHandle},
  time::Duration,
};

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{
  entities::{Action, PeerId, PeerInfo},
  storage::{rusqlite::RusqliteStorage, TodoStorage},
  ShizenError, ShizenResult,
};

pub struct SyncConnection {
  stream: TcpStream,
}

impl SyncConnection {
  pub fn new<A: ToSocketAddrs>(addr: &A) -> ShizenResult<SyncConnection> {
    let stream = TcpStream::connect(addr)?;
    Ok(SyncConnection { stream })
  }

  pub fn peer_id_handshake(&mut self, this_peer_id: &PeerId) -> ShizenResult<PeerId> {
    let response = sync_protocol_tx(
      &mut self.stream,
      &SyncRequest::PeerIdentificationHandshake(this_peer_id.clone()),
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
    peer: &PeerInfo,
    database: &RusqliteStorage,
  ) -> ShizenResult<SyncResults> {
    // TODO
    // 1. Check that peer is added to peers table, if not handshake it.
    let peer = {
      let stored_peer_info = database.get_peer(&peer.peer_id);
      if let Err(_) = stored_peer_info {
        let this_peer_id = database.get_peer_id()?;
        let peer_id = self.peer_id_handshake(&this_peer_id)?;
        database.get_peer(&peer_id)?
      } else {
        stored_peer_info.unwrap()
      }
    };

    // 2. Request all changes since last synced change
    let response = sync_protocol_tx(
      &mut self.stream,
      &SyncRequest::Sync {
        last_sync_clock: peer.clock,
      },
    )?;

    let changes = match response {
      SyncResponse::PeerIdentificationHandshakeResponse(_) => {
        return Err(ShizenError::UnexpectedSyncProtocolResponse(
          "SyncResponse".to_string(),
          response,
        ));
      }
      SyncResponse::SyncResponse { changes } => changes,
    };

    // Insert each of these in the database
    for change_json in &changes {
      let change: Action = serde_json::from_str(&change_json)?;
      database.apply_action(&change)?;
    }

    // 3. Rebase our changes on top of these and increment our version to match.
    // 4. Update the synced version of this peer in the peers table
    // 5. OPTIONAL in the recieving in notify of some way to request sync back.
    todo!()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
  PeerIdentificationHandshake(PeerId),
  Sync { last_sync_clock: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
  PeerIdentificationHandshakeResponse(PeerId),
  SyncResponse {
    /// Vec of encoded change jsons (same as in undo table)
    changes: Vec<String>,
  },
}

pub struct SyncResults {}

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
          Ok((socket, addr)) => {
            if let Err(e) = Self::handle_request(socket, addr, &this_peer_id, &database) {
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
    addr: SocketAddr,
    this_peer_id: &PeerId,
    database: &RusqliteStorage,
  ) -> ShizenResult<()> {
    sync_protocol_rx(&mut stream, &addr, this_peer_id, database)
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
  addr: &SocketAddr,
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
    SyncRequest::PeerIdentificationHandshake(other_peer_id) => {
      let other_clock = 0; // No syncing done yet, always start at 0.
      database.add_connected_peer(other_peer_id, other_clock, addr)?;

      serialize_message_to_stream(
        &SyncResponse::PeerIdentificationHandshakeResponse(this_peer_id.clone()),
        stream,
      )?;
    }
    // TODO now implement sync and test
    SyncRequest::Sync { last_sync_clock } => todo!(),
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
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let first_pid = first.get_peer_id().unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1701", in_memory_uri.into()).unwrap();

    let second = RusqliteStorage::open(None).unwrap();

    let second_pid = second.get_peer_id().unwrap();

    let expected_first_pid = second.add_peer("localhost:1701").unwrap();
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
      SyncServer::listen_on_thread("localhost:1701", in_memory_uri.into()).unwrap();

    let second = RusqliteStorage::open(None).unwrap();
    let sync_peer_id = second.add_peer("localhost:1701").unwrap();
    let peer = second.get_peer(&sync_peer_id).unwrap();

    let picard = first
      .create_new_note("Picard", Some("Captain"), None)
      .unwrap();
    let riker = first
      .create_new_note("Riker", Some("Number One"), Some(&picard.id))
      .unwrap();

    second.sync_with_peer(&peer).unwrap();
  }
}
