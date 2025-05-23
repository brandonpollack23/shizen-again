import { useState, useEffect, useRef } from 'react';
import { useStore, getSelectedNote } from '../store';
import { updateNoteTitle, updateNoteDescription, deleteNote } from '../api';
import { TrashIcon, PencilIcon, CheckIcon, XMarkIcon } from '@heroicons/react/24/outline';

export function TaskDetail() {
  // Get only what we need from the store
  const notes = useStore(state => state.notes);
  const selectedNoteId = useStore(state => state.selectedNoteId);
  
  // Actions
  const updateNote = useStore(state => state.updateNote);
  const removeNote = useStore(state => state.deleteNote);
  const setSelectedNoteId = useStore(state => state.setSelectedNoteId);
  const setError = useStore(state => state.setError);
  
  // Get selected note using our selector function
  const selectedNote = getSelectedNote({ notes, selectedNoteId });
  
  // Local state for editing
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [titleValue, setTitleValue] = useState('');
  const [isEditingDescription, setIsEditingDescription] = useState(false);
  const [descriptionValue, setDescriptionValue] = useState('');
  
  // Refs for input focus
  const titleInputRef = useRef<HTMLInputElement>(null);
  const descriptionInputRef = useRef<HTMLTextAreaElement>(null);
  
  // Update local state when selected note changes
  useEffect(() => {
    if (selectedNote) {
      setTitleValue(selectedNote.title);
      setDescriptionValue(selectedNote.description || '');
    }
  }, [selectedNote]);
  
  // Focus inputs when editing starts
  useEffect(() => {
    if (isEditingTitle && titleInputRef.current) {
      titleInputRef.current.focus();
    }
    if (isEditingDescription && descriptionInputRef.current) {
      descriptionInputRef.current.focus();
    }
  }, [isEditingTitle, isEditingDescription]);
  
  // Show empty state if no note is selected
  if (!selectedNote) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-4 text-gray-500 dark:text-gray-400">
        <p className="text-lg">Select a task to view details</p>
      </div>
    );
  }
  
  // Handle title save
  const handleTitleSave = async () => {
    try {
      // Call backend
      await updateNoteTitle(selectedNote.id[0], titleValue);
      
      // Update store
      updateNote({
        ...selectedNote,
        title: titleValue,
      });
      
      // Exit edit mode
      setIsEditingTitle(false);
    } catch (err) {
      console.error('Error updating title:', err);
      setError(err instanceof Error ? err.message : 'Failed to update title');
    }
  };
  
  // Handle description save
  const handleDescriptionSave = async () => {
    try {
      // Call backend
      await updateNoteDescription(selectedNote.id[0], descriptionValue || undefined);
      
      // Update store
      updateNote({
        ...selectedNote,
        description: descriptionValue || null,
      });
      
      // Exit edit mode
      setIsEditingDescription(false);
    } catch (err) {
      console.error('Error updating description:', err);
      setError(err instanceof Error ? err.message : 'Failed to update description');
    }
  };
  
  // Handle task deletion
  const handleDeleteTask = async () => {
    if (confirm('Are you sure you want to delete this task?')) {
      try {
        // Call backend
        await deleteNote(selectedNote.id[0]);
        
        // Update store
        removeNote(selectedNote.id[0]);
        setSelectedNoteId(null);
      } catch (err) {
        console.error('Error deleting task:', err);
        setError(err instanceof Error ? err.message : 'Failed to delete task');
      }
    }
  };
  
  // Cancel title edit
  const cancelTitleEdit = () => {
    setTitleValue(selectedNote.title);
    setIsEditingTitle(false);
  };
  
  // Cancel description edit
  const cancelDescriptionEdit = () => {
    setDescriptionValue(selectedNote.description || '');
    setIsEditingDescription(false);
  };
  
  return (
    <div className="h-full overflow-y-auto p-4 space-y-4">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          {isEditingTitle ? (
            <div className="flex items-center gap-2">
              <input
                ref={titleInputRef}
                type="text"
                value={titleValue}
                onChange={(e) => setTitleValue(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 dark:bg-gray-700 dark:text-white"
              />
              <button
                onClick={handleTitleSave}
                className="p-2 text-green-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full"
              >
                <CheckIcon className="h-5 w-5" />
              </button>
              <button
                onClick={cancelTitleEdit}
                className="p-2 text-red-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full"
              >
                <XMarkIcon className="h-5 w-5" />
              </button>
            </div>
          ) : (
            <div className="flex items-center group">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{selectedNote.title}</h2>
              <button
                onClick={() => setIsEditingTitle(true)}
                className="ml-2 p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <PencilIcon className="h-4 w-4" />
              </button>
            </div>
          )}
        </div>

        <button
          onClick={handleDeleteTask}
          className="p-2 text-gray-400 hover:text-red-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full"
        >
          <TrashIcon className="h-5 w-5" />
        </button>
      </div>

      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        <div className="flex items-center mb-2">
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300">Description</h3>
          {!isEditingDescription && (
            <button
              onClick={() => setIsEditingDescription(true)}
              className="ml-2 p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            >
              <PencilIcon className="h-4 w-4" />
            </button>
          )}
        </div>

        {isEditingDescription ? (
          <div className="space-y-2">
            <textarea
              ref={descriptionInputRef}
              value={descriptionValue}
              onChange={(e) => setDescriptionValue(e.target.value)}
              className="w-full h-32 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 dark:bg-gray-700 dark:text-white"
              placeholder="Add a description..."
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={cancelDescriptionEdit}
                className="px-3 py-1 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md"
              >
                Cancel
              </button>
              <button
                onClick={handleDescriptionSave}
                className="px-3 py-1 text-sm text-white bg-primary-600 hover:bg-primary-700 rounded-md"
              >
                Save
              </button>
            </div>
          </div>
        ) : (
          <p className="text-gray-600 dark:text-gray-400 whitespace-pre-wrap">
            {selectedNote.description || 'No description provided.'}
          </p>
        )}
      </div>

      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Subtasks</h3>
        {selectedNote.children_ids.length === 0 ? (
          <p className="text-gray-500 dark:text-gray-400 text-sm">No subtasks.</p>
        ) : (
          <ul className="space-y-1">
            <li className="text-gray-500 dark:text-gray-400 text-sm">
              {selectedNote.children_ids.length} subtask(s)
            </li>
          </ul>
        )}
      </div>

      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Blocked By</h3>
        {selectedNote.notes_blocking_this.length === 0 ? (
          <p className="text-gray-500 dark:text-gray-400 text-sm">Not blocked by any tasks.</p>
        ) : (
          <ul className="space-y-1">
            <li className="text-gray-500 dark:text-gray-400 text-sm">
              {selectedNote.notes_blocking_this.length} blocker(s)
            </li>
          </ul>
        )}
      </div>

      <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Blocks</h3>
        {selectedNote.notes_this_blocks.length === 0 ? (
          <p className="text-gray-500 dark:text-gray-400 text-sm">Doesn't block any tasks.</p>
        ) : (
          <ul className="space-y-1">
            <li className="text-gray-500 dark:text-gray-400 text-sm">
              {selectedNote.notes_this_blocks.length} task(s)
            </li>
          </ul>
        )}
      </div>
    </div>
  );
}