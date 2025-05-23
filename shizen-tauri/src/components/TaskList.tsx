import { useEffect } from 'react';
import { useStore, getFilteredNotes } from '../store';
import { CheckCircleIcon, MinusCircleIcon, ChevronRightIcon } from '@heroicons/react/24/outline';
import { CheckCircleIcon as CheckCircleSolidIcon } from '@heroicons/react/24/solid';
import { setNoteCompleted, fetchAllNotes } from '../api';

export function TaskList() {
  const isLoading = useStore(state => state.isLoading);
  const error = useStore(state => state.error);
  const selectedNoteId = useStore(state => state.selectedNoteId);
  const notes = useStore(state => state.notes);
  const filter = useStore(state => state.filter);
  
  const setSelectedNoteId = useStore(state => state.setSelectedNoteId);
  const setError = useStore(state => state.setError);
  const setIsLoading = useStore(state => state.setIsLoading);
  const setNotes = useStore(state => state.setNotes);
  
  const filteredNotes = getFilteredNotes({ notes, filter });
  
  useEffect(() => {
    async function loadNotes() {
      try {
        setIsLoading(true);
        const response = await fetchAllNotes();
        setNotes(response);
      } catch (err) {
        console.error('Error loading notes:', err);
        setError(err instanceof Error ? err.message : 'Failed to load notes');
      } finally {
        setIsLoading(false);
      }
    }
    
    loadNotes();
  }, [setNotes, setIsLoading, setError]);
  
  const handleToggleComplete = async (noteId: string, completed: boolean, e: React.MouseEvent) => {
    e.stopPropagation();
    
    try {
      await setNoteCompleted(noteId, !completed);
      
      const updatedNotes = await fetchAllNotes();
      setNotes(updatedNotes);
    } catch (err) {
      console.error('Error toggling completion:', err);
      setError(err instanceof Error ? err.message : 'Failed to update note');
    }
  };
  
  // Handle selecting a note
  const handleSelectNote = (noteId: string) => {
    setSelectedNoteId(noteId);
  };
  
  // Show loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-primary-500"></div>
      </div>
    );
  }
  
  // Show error state
  if (error) {
    return (
      <div className="p-4 text-red-500">
        Error: {error}
      </div>
    );
  }
  
  // Show empty state
  if (filteredNotes.length === 0) {
    return (
      <div className="p-4 text-center text-gray-500">
        No tasks found. Create a new task to get started.
      </div>
    );
  }
  
  // Show list of tasks
  return (
    <div className="overflow-y-auto">
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {filteredNotes.map((note) => (
          <li 
            key={note.id[0]}
            onClick={() => handleSelectNote(note.id[0])}
            className={`p-4 hover:bg-gray-100 dark:hover:bg-gray-800 cursor-pointer ${
              selectedNoteId === note.id[0] ? 'bg-gray-100 dark:bg-gray-800' : ''
            }`}
          >
            <div className="flex items-center gap-3">
              <button 
                onClick={(e) => handleToggleComplete(note.id[0], note.completed, e)}
                className="flex-shrink-0"
              >
                {note.completed ? (
                  <CheckCircleSolidIcon className="h-6 w-6 text-green-500" />
                ) : (
                  <CheckCircleIcon className="h-6 w-6 text-gray-400" />
                )}
              </button>
              
              <div className="flex-1 min-w-0">
                <p className={`text-sm font-medium truncate ${
                  note.completed ? 'line-through text-gray-500' : 'text-gray-900 dark:text-white'
                }`}>
                  {note.title}
                </p>
                
                {note.description && (
                  <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
                    {note.description}
                  </p>
                )}
                
                <div className="flex mt-1 gap-2 text-xs">
                  {note.notes_blocking_this.length > 0 && (
                    <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200">
                      <MinusCircleIcon className="h-3 w-3" />
                      Blocked
                    </span>
                  )}
                  
                  {note.children_ids.length > 0 && (
                    <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                      <ChevronRightIcon className="h-3 w-3" />
                      {note.children_ids.length} {note.children_ids.length === 1 ? 'subtask' : 'subtasks'}
                    </span>
                  )}
                </div>
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}