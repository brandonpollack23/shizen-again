-- By default rank can be empty and empty ranks will always be displayed first.
ALTER TABLE Notes
ADD COLUMN rank TEXT NOT NULL DEFAULT "";
CREATE INDEX IF NOT EXISTS NotesUserRank
ON Notes (rank);
