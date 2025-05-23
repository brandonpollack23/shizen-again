import { z } from 'zod';

// Define schema for NoteId
export const NoteIdSchema = z.object({
  0: z.string().uuid(),
});

export type NoteId = z.infer<typeof NoteIdSchema>;

// Define schema for Note
export const NoteSchema = z.object({
  id: NoteIdSchema,
  title: z.string(),
  description: z.string().nullable().optional(),
  completed: z.boolean(),
  parent_id: NoteIdSchema.nullable().optional(),
  children_ids: z.array(NoteIdSchema),
  notes_this_blocks: z.array(NoteIdSchema),
  notes_blocking_this: z.array(NoteIdSchema),
});

export type Note = z.infer<typeof NoteSchema>;

// Define schema for PeerId
export const PeerIdSchema = z.object({
  0: z.string().uuid(),
});

export type PeerId = z.infer<typeof PeerIdSchema>;

// Define schema for PeerInfo
export const PeerInfoSchema = z.object({
  peer_id: PeerIdSchema,
  clock: z.number(),
  addr: z.string(), // Socket address will be a string in the frontend
});

export type PeerInfo = z.infer<typeof PeerInfoSchema>;

// Filter types
export type NoteFilter = {
  showCompleted: boolean;
  showBlocked: boolean;
  searchTerm: string;
  parentId?: string | null;
};

// Action types that can be performed on notes
export enum ActionType {
  CREATE_NOTE = 'CreateNote',
  SET_COMPLETED = 'SetCompleted',
  UPDATE_TITLE = 'UpdateTitle',
  UPDATE_DESCRIPTION = 'UpdateDescription',
  CHANGE_PARENT = 'ChangeParent',
  ADD_DEPENDENCY = 'AddDependency',
  REMOVE_DEPENDENCY = 'RemoveDependency',
  DELETE_NOTE = 'DeleteNote',
  REORDER_NOTE = 'ReorderNote',
}