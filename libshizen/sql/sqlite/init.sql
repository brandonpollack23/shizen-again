-- Initializes all the tables required for shizen.

CREATE TABLE IF NOT EXISTS SchemaVersion (
  id INTEGER PRIMARY KEY CHECK(ID = 1),
  version INTEGER
);
INSERT INTO SchemaVersion
VALUES (1, 1);

CREATE TABLE IF NOT EXISTS Notes (
  uuid TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  parent_id TEXT
);

CREATE TABLE IF NOT EXISTS Children (
  parent TEXT NOT NULL,
  child TEXT NOT NULL
  -- TODO
  -- FOREIGN KEY(parent) REFERENCES Notes(uuid),
  -- FOREIGN KEY(child) REFERENCES Notes(uuid)
);
CREATE INDEX IF NOT EXISTS ParentToChildIndex
ON Children (parent)
