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
  description TEXT
);

CREATE TABLE IF NOT EXISTS Children (
  parent TEXT NOT NULL,
  child TEXT NOT NULL
  -- TODO
  -- FOREIGN KEY(parent) REFERENCES Notes(uuid),
  -- FOREIGN KEY(child) REFERENCES Notes(uuid)
);
CREATE INDEX IF NOT EXISTS ParentToChildIndex
ON Children (parent);
CREATE INDEX IF NOT EXISTS ParentToChildIndex
ON Children (child);

CREATE TABLE IF NOT EXISTS Dependencies (
  blocker TEXT NOT NULL,
  blockee TEXT NOT NULL
  -- TODO
  -- FOREIGN KEY(parent) REFERENCES Notes(uuid),
  -- FOREIGN KEY(child) REFERENCES Notes(uuid)
);
CREATE INDEX IF NOT EXISTS BlockerToBlockeeIndex
ON Dependencies (blocker);
CREATE INDEX IF NOT EXISTS BlockeeToBlockerIndex
ON Dependencies (blockee);

CREATE VIEW IF NOT EXISTS FullyQualifiedNotes AS 
SELECT 
  uuid,
  title,
  description,
  parent,
  GROUP_CONCAT(blocks.blockee, ',') AS blocks,
  GROUP_CONCAT(blocked.blocker, ',') AS blocked
FROM Notes AS n
LEFT JOIN Children AS c ON n.uuid = c.child
LEFT JOIN Dependencies AS blocks ON n.uuid = blocks.blocker
LEFT JOIN Dependencies AS blocked ON n.uuid = blocks.blockee
GROUP BY uuid, title, description, parent;
