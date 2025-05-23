import { invoke } from '@tauri-apps/api/core';
import { Note, PeerInfo } from './types';

// Helper to wrap invoke with try/catch
async function invokeCommand(command: string, args?: Record<string, any>): Promise<any> {
  try {
    const result = await invoke(command, args);
    return result;
  } catch (error) {
    console.error(`Error invoking ${command}:`, error);
    throw error;
  }
}

// Note APIs
export async function fetchAllNotes(): Promise<Note[]> {
  return invokeCommand('load_all_notes');
}

export async function fetchUnblockedNotes(showCompleted: boolean): Promise<Note[]> {
  return invokeCommand('load_all_unblocked_notes', { 
    load_completed: showCompleted // Match Rust parameter name
  });
}

export async function createNote(
  title: string, 
  description?: string, 
  parentId?: string
): Promise<Note> {
  return invokeCommand('create_new_note', { 
    title, 
    description: description || null,
    parent: parentId || null
  });
}

export async function updateNoteTitle(
  noteId: string, 
  title: string
): Promise<void> {
  return invokeCommand('update_title', { 
    note_id: noteId, // Match Rust parameter name 
    title 
  });
}

export async function updateNoteDescription(
  noteId: string, 
  description?: string
): Promise<void> {
  return invokeCommand('update_description', { 
    note_id: noteId, // Match Rust parameter name
    description: description || null
  });
}

export async function updateNoteParent(
  noteId: string, 
  parentId?: string
): Promise<void> {
  return invokeCommand('update_parent', { 
    note_id: noteId, // Match Rust parameter name
    parent: parentId || null
  });
}

export async function setNoteCompleted(
  noteId: string, 
  completed: boolean
): Promise<void> {
  return invokeCommand('set_completed', { 
    note_id: noteId, // Match Rust parameter name
    completed 
  });
}

export async function deleteNote(noteId: string): Promise<void> {
  return invokeCommand('delete_note', { 
    note_id: noteId  // Match Rust parameter name
  });
}

export async function addDependency(
  blockerId: string, 
  blockeeId: string
): Promise<void> {
  return invokeCommand('add_blocked_note', { 
    note_id: blockerId, // Match Rust parameter name
    blocked_note: blockeeId // Match Rust parameter name
  });
}

export async function removeDependency(
  blockerId: string, 
  blockeeId: string
): Promise<void> {
  return invokeCommand('remove_blocked_note', { 
    note_id: blockerId, // Match Rust parameter name
    blocked_note: blockeeId // Match Rust parameter name
  });
}

export async function reorderNote(
  noteId: string,
  beforeId?: string,
  afterId?: string
): Promise<void> {
  return invokeCommand('adjust_rank_between', {
    note_id: noteId, // Match Rust parameter name
    before: beforeId || null,
    after: afterId || null
  });
}

// Sync APIs
export async function fetchPeers(): Promise<PeerInfo[]> {
  return invokeCommand('get_peers');
}

export async function addPeer(
  address: string, 
  localServerPort?: number
): Promise<string> {
  return invokeCommand('add_peer', { 
    addr: address,
    local_server_port: localServerPort || null // Match Rust parameter name
  });
}

export async function removePeer(peerId: string): Promise<void> {
  return invokeCommand('remove_peer', { 
    peer_id: peerId // Match Rust parameter name
  });
}

export async function syncWithPeer(peerId: string): Promise<{
  updatedPeerClock: number;
  numChanges: number;
}> {
  return invokeCommand('sync_with_peer', { 
    peer_id: peerId // Match Rust parameter name
  });
}

export async function startSyncServer(port: number): Promise<void> {
  return invokeCommand('start_sync_server', { port });
}

// Undo/Redo APIs
export async function undo(): Promise<number> {
  return invokeCommand('undo');
}

export async function redo(): Promise<number> {
  return invokeCommand('redo');
}