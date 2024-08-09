-- Initializes all the tables required for shizen.

CREATE TABLE IF NOT EXISTS SchemaVersion (
  id INTEGER PRIMARY KEY CHECK(ID = 1),
  version INTEGER
);
INSERT INTO SchemaVersion
VALUES (1, 1);

-- Single row table of settings.
CREATE TABLE IF NOT EXISTS LocalSettings (
  -- The "version" of the database locally (used for sync).
  clock INTEGER NOT NULL,
  peer_id STRING NOT NULL
);

CREATE TABLE IF NOT EXISTS Notes (
  uuid TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  completed INTEGER NOT NULL
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
  n.uuid,
  n.title,
  n.description,
  parent.parent,
  GROUP_CONCAT(blocks.blockee, ',') AS blocks,
  GROUP_CONCAT(blocked.blocker, ',') AS blocked,
  GROUP_CONCAT(children.child, ',') AS children,
  n.completed,
  IFNULL(MAX(NOT blocker_done.completed), 0) AS is_blocked -- MAX on ints is actually BOOLEAN AND
FROM Notes AS n
LEFT JOIN Children AS parent ON n.uuid = parent.child
LEFT JOIN Children AS children ON n.uuid = children.parent
LEFT JOIN Dependencies AS blocks ON n.uuid = blocks.blocker
LEFT JOIN Dependencies AS blocked ON n.uuid = blocked.blockee
LEFT JOIN Notes AS blocker_done ON blocked.blocker = blocker_done.uuid
GROUP BY n.uuid, n.title, n.description, children.parent, n.completed;

CREATE TABLE IF NOT EXISTS Mutations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action_json TEXT NOT NULL,
  clock INTEGER NOT NULL
);

-- TODO clear in maintain method.
CREATE TABLE IF NOT EXISTS RedoMutations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS Peers (
  peer_id STRING NOT NULL,
  clock INTEGER NOT NULL,
  addr STRING NOT NULL
);
CREATE INDEX IF NOT EXISTS PeerIdIndex
ON Peers (peer_id);

