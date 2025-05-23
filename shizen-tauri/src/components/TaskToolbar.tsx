import { useState } from 'react';
import { useStore } from '../store';
import { PlusIcon, FunnelIcon, ArrowUturnLeftIcon, ArrowUturnRightIcon } from '@heroicons/react/24/outline';
import { createNote, undo, redo } from '../api';

interface TaskToolbarProps {
  onOpenFilter: () => void;
}

export function TaskToolbar({ onOpenFilter }: TaskToolbarProps) {
  // Get actions from store
  const addNote = useStore(state => state.addNote);
  const setError = useStore(state => state.setError);
  const setIsLoading = useStore(state => state.setIsLoading);
  
  // Local state
  const [isCreatingTask, setIsCreatingTask] = useState(false);

  // Handle creating a new task
  const handleCreateTask = async () => {
    try {
      setIsCreatingTask(true);
      setIsLoading(true);
      
      // Call backend
      const newNote = await createNote('New Task');
      
      // Update store
      addNote(newNote);
    } catch (err) {
      console.error('Error creating task:', err);
      setError(err instanceof Error ? err.message : 'Failed to create task');
    } finally {
      setIsLoading(false);
      setIsCreatingTask(false);
    }
  };

  // Handle undo
  const handleUndo = async () => {
    try {
      // Call backend
      await undo();
      
      // Reload notes after undo
      const notesData = await window.__TAURI__.invoke('load_all_notes');
      
      // Update store
      useStore.getState().setNotes(notesData);
    } catch (err) {
      console.error('Error undoing:', err);
      setError(err instanceof Error ? err.message : 'Failed to undo');
    }
  };

  // Handle redo
  const handleRedo = async () => {
    try {
      // Call backend
      await redo();
      
      // Reload notes after redo
      const notesData = await window.__TAURI__.invoke('load_all_notes');
      
      // Update store
      useStore.getState().setNotes(notesData);
    } catch (err) {
      console.error('Error redoing:', err);
      setError(err instanceof Error ? err.message : 'Failed to redo');
    }
  };

  return (
    <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 p-2 flex justify-between items-center">
      <div className="flex items-center gap-2">
        <button
          onClick={handleUndo}
          className="p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300"
          title="Undo"
        >
          <ArrowUturnLeftIcon className="h-5 w-5" />
        </button>
        <button
          onClick={handleRedo}
          className="p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300"
          title="Redo"
        >
          <ArrowUturnRightIcon className="h-5 w-5" />
        </button>
      </div>

      <div className="flex items-center gap-2">
        <button
          onClick={onOpenFilter}
          className="p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300"
          title="Filter Tasks"
        >
          <FunnelIcon className="h-5 w-5" />
        </button>
        <button
          onClick={handleCreateTask}
          disabled={isCreatingTask}
          className="flex items-center gap-1 px-3 py-1.5 rounded-full bg-primary-600 text-white hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500 disabled:opacity-50"
        >
          <PlusIcon className="h-4 w-4" />
          <span className="text-sm font-medium">New Task</span>
        </button>
      </div>
    </div>
  );
}