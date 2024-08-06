use std::{
  io::{Read, Write},
  net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
  sync::{mpsc::Sender, Arc},
  thread::{self, JoinHandle},
  time::Duration,
};

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{
  entities::PeerId,
  storage::{rusqlite::RusqliteStorage, TodoStorage},
  ShizenError, ShizenResult,
};

// TODO NOW peers and sync.
// implement sync server side.
// then sync algorithm.

pub struct SyncConnection {
  stream: TcpStream,
}

impl SyncConnection {
  pub fn new<A: ToSocketAddrs>(addr: A) -> ShizenResult<SyncConnection> {
    let stream = TcpStream::connect(addr)?;
    Ok(SyncConnection { stream })
  }

  pub fn peer_id_handshake(
    &mut self,
    this_peer_id: PeerId,
    current_clock: usize,
  ) -> ShizenResult<(PeerId, usize)> {
    let response = sync_protocol_tx(
      &mut self.stream,
      &SyncRequest::PeerIdentificationHandshake((this_peer_id, current_clock)),
    )?;

    match response {
      SyncResponse::PeerIdentificationHandshakeResponse(r) => return Ok(r),
      other => {
        return Err(ShizenError::UnexpectedSyncProtocolResponse(
          "PeerIdentificationHandshakeResponse".to_string(),
          other,
        ))
      }
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum SyncRequest {
  PeerIdentificationHandshake((PeerId, usize)),
  Sync { last_sync_clock: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum SyncResponse {
  PeerIdentificationHandshakeResponse((PeerId, usize)),
  SyncResponse,
}

// TODO spawn threads to sync multiple peers at once?

/// Server that will listen on sync connections (blocking) as long as it is constructed.
pub struct SyncServer {
  thread: JoinHandle<()>,
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

    let thread = thread::spawn(move || {
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
    });

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
    self.kill_tx.send(());
  }
}

fn sync_protocol_tx(
  stream: &mut TcpStream,
  sync_message: &SyncRequest,
) -> Result<SyncResponse, ShizenError> {
  serialize_message_to_stream(sync_message, stream)?;

  let mut response_len_bytes = [0; 4];
  stream.read_exact(&mut response_len_bytes);
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
  stream.write(&message.as_bytes())?;
  Ok(())
}

fn sync_protocol_rx(
  stream: &mut TcpStream,
  addr: &SocketAddr,
  this_peer_id: &PeerId,
  database: &RusqliteStorage,
) -> ShizenResult<()> {
  let mut request_len_bytes = [0; 4];
  stream.read_exact(&mut request_len_bytes);
  let request_len = u32::from_be_bytes(request_len_bytes) as usize;
  let mut request_bytes = vec![0; request_len];
  stream.read_exact(&mut request_bytes);

  let request: SyncRequest = serde_json::from_slice(&request_bytes)?;

  match request {
    SyncRequest::PeerIdentificationHandshake((other_peer_id, other_clock)) => {
      database.add_connected_peer(other_peer_id, other_clock, addr)?;

      let current_clock = database.get_clock()?;
      serialize_message_to_stream(
        &SyncResponse::PeerIdentificationHandshakeResponse((this_peer_id.clone(), current_clock)),
        stream,
      )?;
    }
    SyncRequest::Sync { last_sync_clock } => todo!(),
  }

  Ok(())
}

// TODO write tests for syncing peers.
#[cfg(test)]
mod test {
  use tracing_test::traced_test;

  use crate::{
    entities::PeerInfo,
    storage::{rusqlite::RusqliteStorage, TodoStorage},
  };

  use super::SyncServer;

  #[test]
  #[traced_test]
  fn peer_syncs() {
    // use uri to share in memory database data https://sqlite.org/inmemorydb.html
    let in_memory_uri = "file::memory:?cache=shared";
    let first = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();
    let first_pid = first.get_peer_id().unwrap();
    let _first_syncer =
      SyncServer::listen_on_thread("localhost:1701", in_memory_uri.into()).unwrap();

    let second = RusqliteStorage::open(Some(&in_memory_uri.into())).unwrap();

    let second_pid = second.get_peer_id().unwrap();

    let expected_first_pid = second.add_peer("localhost:1701").unwrap();
    assert_eq!(expected_first_pid, first_pid);

    let second_peer_read = &first.get_peers().unwrap()[0];
    assert_eq!(second_peer_read.peer_id, second_pid);
    assert_eq!(second_peer_read.clock, 0);
  }
}
