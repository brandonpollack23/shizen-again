-- Initializes all the tables required for shizen.

CREATE TABLE IF NOT EXISTS SchemaVersion (
  ID INTEGER PRIMARY KEY CHECK(ID = 1),
  VERSION INTEGER
);
INSERT INTO SchemaVersion
VALUES (1, 1);

CREATE TABLE IF NOT EXISTS Todos (
  uuid TEXT PRIMARY KEY,
  title TEXT,
  description TEXT,
  parent_id TEXT
);
