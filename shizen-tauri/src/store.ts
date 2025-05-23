import { create } from 'zustand';
import { Note, NoteFilter, PeerInfo } from './types';

// Define the store state
interface NoteState {
  // Data
  notes: Note[];
  selectedNoteId: string | null;
  filter: NoteFilter;
  peers: PeerInfo[];
  isLoading: boolean;
  error: string | null;
  
  // Actions
  setNotes: (notes: Note[]) => void;
  addNote: (note: Note) => void;
  updateNote: (note: Note) => void;
  deleteNote: (noteId: string) => void;
  setSelectedNoteId: (noteId: string | null) => void;
  setFilter: (filter: Partial<NoteFilter>) => void;
  setPeers: (peers: PeerInfo[]) => void;
  addPeer: (peer: PeerInfo) => void;
  removePeer: (peerId: string) => void;
  setIsLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
}

// Create the store
export const useStore = create<NoteState>()((set) => ({
  // Initial state
  notes: [],
  selectedNoteId: null,
  filter: {
    showCompleted: false,
    showBlocked: false,
    searchTerm: '',
    parentId: null,
  },
  peers: [],
  isLoading: false,
  error: null,

  // Actions
  setNotes: (notes) => set({ notes }),
  
  addNote: (note) => set((state) => ({
    notes: [...state.notes, note],
  })),
  
  updateNote: (updatedNote) => set((state) => ({
    notes: state.notes.map((note) => 
      note.id[0] === updatedNote.id[0] ? updatedNote : note
    ),
  })),
  
  deleteNote: (noteId) => set((state) => ({
    notes: state.notes.filter((note) => note.id[0] !== noteId),
    selectedNoteId: state.selectedNoteId === noteId ? null : state.selectedNoteId,
  })),
  
  setSelectedNoteId: (noteId) => set({ selectedNoteId: noteId }),
  
  setFilter: (filter) => set((state) => ({
    filter: { ...state.filter, ...filter },
  })),
  
  setPeers: (peers) => set({ peers }),
  
  addPeer: (peer) => set((state) => ({
    peers: [...state.peers, peer],
  })),
  
  removePeer: (peerId) => set((state) => ({
    peers: state.peers.filter((peer) => peer.peer_id[0] !== peerId),
  })),
  
  setIsLoading: (isLoading) => set({ isLoading }),
  
  setError: (error) => set({ error }),
}));

// Helper selector functions that can be used outside of React components
export const getFilteredNotes = ({ notes, filter }: { notes: Note[], filter: NoteFilter }) => {
  return notes.filter(note => {
    // Filter by completion status
    if (!filter.showCompleted && note.completed) {
      return false;
    }

    // Filter by blocked status
    if (!filter.showBlocked && note.notes_blocking_this.length > 0) {
      return false;
    }

    // Filter by parent
    if (filter.parentId !== undefined) {
      const hasMatchingParent = filter.parentId === null 
        ? note.parent_id === null
        : note.parent_id && note.parent_id[0] === filter.parentId;

      if (!hasMatchingParent) {
        return false;
      }
    }

    // Filter by search term
    if (filter.searchTerm) {
      const searchLower = filter.searchTerm.toLowerCase();
      const titleMatch = note.title.toLowerCase().includes(searchLower);
      const descMatch = note.description 
        ? note.description.toLowerCase().includes(searchLower) 
        : false;
      
      if (!titleMatch && !descMatch) {
        return false;
      }
    }

    return true;
  });
};

export const getSelectedNote = ({ notes, selectedNoteId }: { notes: Note[], selectedNoteId: string | null }) => {
  if (!selectedNoteId) return null;
  return notes.find(note => note.id[0] === selectedNoteId) || null;
};