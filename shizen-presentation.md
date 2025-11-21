---
title: "Shizen: Local-First P2P Synced Todo Management"
author: Brandon Pollack
---

# Shizen
## Local-First P2P Todo Management

_自然 (Shizen) - "Natural" in Japanese_
_死前 (Shizen) - "Before (Your) Death" in Japanese_

---

# About Me

**Brandon Pollack**

* VP of TokyoRust.org
* Tokyo based software consultant/architect/speaker

## Social

* [GitHub](https://github.com/brandonpollack23)
* [LinkedIn](https://www.linkedin.com/in/brandon-pollack-58b61542/)
* [Email](mailto:brandonpollack23@gmail.com)

## Bio

Former software engineer at Microsoft, Google, and Pulumi.  Worked on all manner of things from
Windows to ChromeOS to distributed systems to Google Keep.

Currently a part time consultant based in Tokyo.

---

# What is Shizen?

A **peer-to-peer synchronized** hierarchical note/todo management system

Written in **Rust** 🦀

Key features:
- Local-first architecture
- Distributed P2P synchronization (no central server!)
- Hierarchical notes with parent-child relationships
- Dependency/blocking relationships like in project management systems (DAG structure)
- Full undo/redo with complete mutation history
- Custom ordering via LexoRank algorithm

---

# The Problem

Modern todo apps have issues:

- ❓➡️❓ **Ordering of Tasks** - What's next? What Can I do now? HALP!?
- ☁️ **Cloud dependency** - Can't work offline, need internet
- 🔒 **Vendor lock-in** - Your data trapped in their ecosystem
- 👁️ **Privacy concerns** - Your tasks on someone else's server
- 💸 **Subscription fatigue** - Monthly fees for basic features
- 🚫 **Service shutdown risk** - What happens when the startup fails?

---

# The Solution: Local-First

Your data lives on **your machines**

```
┌─────────────────────────────────────┐
│  Your Computer (Source of Truth)   │
│  ┌───────────────────────────────┐ │
│  │     ~/shizen.db (SQLite)      │ │
│  │   All your notes, locally     │ │
│  └───────────────────────────────┘ │
└─────────────────────────────────────┘
         │              │
         ↓              ↓
    Sync when      No internet?
    you want!      No problem!
```

**Benefits of local-first:**
- **Offline-first:** Works without internet (airplane mode!)
- **Performance:** No network latency for local operations
- **Privacy:** Your data never leaves your machine (unless you sync)
- **Ownership:** You control your data, backups, migrations
- **Reliability:** No dependency on third-party servers
- **Cost:** No cloud hosting fees

**You own your data. You control the sync.**

---

# Anti-Cloud Philosophy

**Why avoid cloud services?**

💰 **Economic:** Subscription models extract recurring revenue
  - $10/month × 10 years = $1,200 for TODO app!

🔒 **Control:** Cloud providers can:
  - Change pricing arbitrarily
  - Shut down services (RIP Google Reader, etc.)
  - Change features you depend on
  - Lock you out of your account

🕵️ **Privacy:** Your data on their servers means:
  - Potential data breaches
  - Government surveillance (PRISM, etc.)
  - Data mining for ads/ML training

**Self-hosting gives you back control!**

**Shizen is designed to work WITHOUT any cloud service**

---

# Why Rust?

Perfect language for this project:

🛡️ **Memory Safety**
- No null pointer dereferences
- No use-after-free bugs
- Thread safety guaranteed at compile time

⚡ **Performance**
- Zero-cost abstractions
- No garbage collection pauses
- C/C++ level performance

🔧 **Tooling**
- Cargo: best-in-class package manager
- Clippy: helpful lints
- rustfmt: automatic formatting

🌐 **Cross-platform**
- Compiles to native, WASM, embedded
- Same code runs on Linux/Mac/Windows

---

# Directory Structure

```
shizen/
├── libshizen/              # 🎯 Core library
│   ├── src/
│   │   ├── lib.rs         # Public API
│   │   ├── entities.rs    # Data structures
│   │   ├── storage.rs     # Storage trait
│   │   │   └── rusqlite.rs # SQLite implementation
│   │   ├── sync.rs        # P2P sync protocol
│   │   └── result.rs      # Error types
│   ├── sql/sqlite/        # Schema & migrations
│   │   ├── init.sql
│   │   └── migrations/
│   └── Cargo.toml
├── shizen-cli/            # CLI (WIP)
├── res/                   # Resources (icons)
└── Cargo.toml             # Workspace config
```

**Currently:** libshizen is the focus (library-only)

---

# libshizen: A Reusable Core

**All the logic lives in libshizen** - use it however you want!

**Option 1: Use the library from any program**
```rust
use libshizen::{RusqliteStorage, TodoStorage, NoteId};

// Automate task creation from your scripts!
let mut storage = RusqliteStorage::new("~/todos.db").unwrap();
storage.create_note(NoteId::new(), "Deploy to prod".into(), None);
```

**Option 2: Build your own UI**
- Tauri/Dioxus desktop app
- Ratatui TUI
- Web frontend (WASM)
- Mobile app (via Rust FFI)

**The power of separation: libshizen doesn't care how you use it!**

---

# P2P Architecture Overview

No central server needed!

```
    Peer A              Peer B              Peer C
  ┌─────────┐         ┌─────────┐         ┌─────────┐
  │ shizen  │ ←─────→ │ shizen  │ ←─────→ │ shizen  │
  │ SQLite  │         │ SQLite  │         │ SQLite  │
  └─────────┘         └─────────┘         └─────────┘
      ↑                                        ↑
      └────────────────────────────────────────┘
           Direct TCP connection (Port 1701)
```

Each peer:
- Maintains complete local copy
- Can sync with any other peer directly
- Works 100% offline
- Syncs whenever you want

---

# Architecture Layers

```
┌─────────────────────────────────────────────────────┐
│              Future UI Layer (TODO)                  │
│   ┌──────────────────┬──────────────────────────┐   │
│   │ Tauri/Dioxus GUI │  Ratatui TUI             │   │
│   └──────────────────┴──────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────┐
│              libshizen (Core Library)                │
├─────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │
│  │   Entities   │  │   Storage    │  │   Sync    │ │
│  │   (Models)   │  │   (Trait)    │  │ (Protocol)│ │
│  └──────────────┘  └──────────────┘  └───────────┘ │
│         │                  │                │        │
│         └──────────────────┴────────────────┘        │
│                            │                         │
│                    ┌───────▼────────┐                │
│                    │ RusqliteStorage│                │
│                    │ (Implements    │                │
│                    │  TodoStorage)  │                │
│                    └────────────────┘                │
└─────────────────────────────────────────────────────┘
                         ▼
┌─────────────────────────────────────────────────────┐
│                 SQLite Database                      │
│              (WAL mode, ~/shizen.db)                 │
└─────────────────────────────────────────────────────┘
```

---

# Data Model: The Note Entity

From `libshizen/src/entities.rs`:

```rust
pub struct Note {
    pub id: NoteId,                        // UUID
    pub title: String,                     // Note title
    pub description: Option<String>,       // Optional details
    pub completed: bool,                   // Done?
    pub parent_id: Option<NoteId>,        // Hierarchy
    pub children_ids: Vec<NoteId>,        // Child notes
    pub notes_this_blocks: Vec<NoteId>,   // I block these
    pub notes_blocking_this: Vec<NoteId>, // These block me
}
```

**Two relationship types:**
1. **Hierarchical** (parent/child) - Tree structure
2. **Dependencies** (blocker/blockee) - DAG structure

---

# Hierarchical + DAG Structure

Notes form both a **tree** AND a **directed acyclic graph**:

```
                Root Todo
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
   Subtask A   Subtask B   Subtask C
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
     Task B1     Task B2     Task B3

   Dependency relationships (separate from hierarchy):

   Task B1 blocks──┐
                   |
   Task C1 blocks  |
                   ▼
   Task A2 ────→ [Cannot complete (optionally even show) until C1 and B1 done]
```

This lets you model:
- Projects with sub-tasks (hierarchy)
- Task dependencies (DAG), e.g. You need to get your driver's license
  translated by JAF and get a Residence certificate from the ward office before
  you can transfer your drivers license.

---

# Actions: Operation-Based Replication and Logging/History

Every mutation is an `Action` enum:

```rust
pub enum Action {
    CreateNote { id: NoteId, title: String, description: Option<String> },
    SetCompleted { id: NoteId, completed: bool },
    UpdateTitle { id: NoteId, new_title: String },
    UpdateDescription { id: NoteId, new_description: Option<String> },
    ChangeParent { id: NoteId, new_parent_id: Option<NoteId> },
    AddDependency { blocker: NoteId, blockee: NoteId },
    RemoveDependency { blocker: NoteId, blockee: NoteId },
    DeleteNote { id: NoteId },
    ReorderNote { id: NoteId, rank: String },
}
```

**Why actions matter:**
- Perfect undo/redo (replay actions forward/backward)
- Sync protocol (send actions, not state)
- Audit log (what changed when)

---

# Storage Layer: Trait Abstraction

`libshizen/src/storage.rs` defines `TodoStorage` trait:

```rust
pub trait TodoStorage {
    fn create_note(&mut self, id: NoteId, title: String,
                   description: Option<String>) -> ShizenResult<()>;
    fn set_completed(&mut self, id: NoteId, completed: bool) -> ShizenResult<()>;
    fn update_title(&mut self, id: NoteId, new_title: String) -> ShizenResult<()>;
    fn get_note(&self, id: NoteId) -> ShizenResult<Note>;
    fn all_notes(&self) -> ShizenResult<Vec<Note>>;
    fn undo(&mut self) -> ShizenResult<()>;
    fn redo(&mut self) -> ShizenResult<()>;
    fn apply_action(&mut self, action: Action) -> ShizenResult<()>;
    // ... and more
}
```

**Benefits:**
- Swap storage backends (SQLite, Postgres, in-memory)
- Easy testing with mock implementations

---

# SQLite Schema: Tables

From `libshizen/sql/sqlite/init.sql`:

**Core Tables:**
1. `SchemaVersion` - Migration tracking
2. `LocalSettings` - Peer ID, Lamport clock (single row)
3. `Notes` - Main note storage (uuid, title, description, completed, rank)
4. `Children` - Parent-child relationships (parent, child)
5. `Dependencies` - Blocking relationships (blocker, blockee)
6. `Mutations` - Action log (id, action_json, clock)
7. `RedoMutations` - Redo queue for undone actions
8. `Peers` - Known sync peers (peer_id, clock, addr)

**Views:**
- `FullyQualifiedNotes` - Denormalized join of all relationships, ordered by rank

---

# SQLite Configuration

```sql
PRAGMA journal_mode = WAL;        -- Write-Ahead Log
PRAGMA synchronous = NORMAL;      -- Performance balance
PRAGMA foreign_keys = ON;         -- Referential integrity
```

**Why WAL (Write-Ahead Logging)?**
- Readers don't block writers
- Writers don't block readers
- Better concurrency for sync scenarios
- Crash recovery

**Foreign keys enforce:**
- Can't have orphaned children
- Can't have dependencies to non-existent notes
- Cascade deletes handled correctly

---

# RusqliteStorage: Implementation

Key implementation details:

**Transaction-based operations:**
```rust
// Every mutation runs in a transaction
let tx = self.conn.transaction()?;
// ... do work ...
tx.commit()?;
```

**Recursive CTEs for hierarchical queries:**
```sql
WITH RECURSIVE descendants AS (
    SELECT uuid FROM Notes WHERE uuid = ?
    UNION ALL
    SELECT c.child FROM Children c
    JOIN descendants d ON c.parent = d.uuid
)
SELECT * FROM Notes WHERE uuid IN descendants;
```

**Comprehensive error handling with `thiserror`:**
```rust
#[derive(Error, Debug)]
pub enum ShizenError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Note not found: {0}")]
    NoteNotFound(NoteId),
    // ... etc
}
```

---

# Hierarchical Queries: Recursive CTEs

**Finding all descendants of a note:**

```sql
WITH RECURSIVE descendants AS (
    SELECT uuid FROM Notes WHERE uuid = ?
    UNION ALL
    SELECT c.child
    FROM Children c
    JOIN descendants d ON c.parent = d.uuid
)
DELETE FROM Notes WHERE uuid IN descendants;
```

**Why recursive CTEs?**
- Handles arbitrary depth hierarchies
- Single SQL query (efficient!)
- Database does the heavy lifting

**Used for:**
- Deleting notes (cascading to descendants)
- Finding all children/grandchildren
- Tree traversals

---

# Dependency Graph: DAG Enforcement

**Dependencies table:**
```sql
CREATE TABLE Dependencies (
    blocker TEXT NOT NULL,
    blockee TEXT NOT NULL,
    FOREIGN KEY (blocker) REFERENCES Notes(uuid) ON DELETE CASCADE,
    FOREIGN KEY (blockee) REFERENCES Notes(uuid) ON DELETE CASCADE
);
```

**TODO:** Cycle detection to ensure DAG property!

```rust
// Should prevent:
add_dependency(A, B)  // A blocks B
add_dependency(B, C)  // B blocks C
add_dependency(C, A)  // C blocks A → CYCLE!
```

**Current:** No cycle prevention (you can create cycles!)
**Future:** Topological sort validation before adding dependency

---

# Error Handling: ShizenError

Using `thiserror` for ergonomic errors:

```rust
#[derive(Error, Debug)]
pub enum ShizenError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Note not found: {0}")]
    NoteNotFound(NoteId),

    #[error("Cannot add self as parent")]
    SelfParentError,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type ShizenResult<T> = Result<T, ShizenError>;
```

**Benefits:** Clear error messages, `?` operator works beautifully

---

# LexoRank: Fractional Indexing

Problem: How to maintain user-defined order without reordering everything?

**Bad approach:**
```
Notes have position: 1, 2, 3, 4, 5
User moves note 5 between 2 and 3
→ Update positions: 1, 2, 5→3, 3→4, 4→5  (3 UPDATEs!)
```

**LexoRank approach:**
```
Notes have rank: "a", "b", "d", "h", "p"
User moves "p" between "b" and "d"
→ New rank: "c" (lexicographically between "b" and "d")
→ Only 1 UPDATE!
```

**When single letters aren't enough:**
```
Notes have rank: "a", "b", "c", "d"
User inserts between "b" and "c"
→ New rank: "bb" (lexicographically: "b" < "bb" < "c")

User inserts again between "b" and "bb"
→ New rank: "bU" (midpoint: "b" < "bU" < "bb")

Infinite precision! You can ALWAYS find a string between any two strings.
"b" < "ba" < "baa" < "baaa" < ... < "bb"
```

**Benefits:**
- O(1) reordering
- Works great with distributed systems (no position conflicts)
- Infinite precision (can always find a value in between)

---

# Undo/Redo: Time Travel

**Every action is logged in `Mutations` table:**

```sql
CREATE TABLE Mutations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_json TEXT,
    clock INTEGER
);
```

**Undo:**
1. Pop last action from `Mutations`
2. Invert it (e.g., CreateNote → DeleteNote)
3. Apply the inverse
4. Push original action to `RedoMutations`

**Redo:**
1. Pop from `RedoMutations`
2. Apply it again
3. Push back to `Mutations`

**Result:** Perfect undo/redo with full history!

---

# P2P Sync: Protocol Overview

Custom **JSON-over-TCP** protocol on port **1701**

```
Message Format:
┌────────────────┬────────────────────────────┐
│ 4-byte length  │    JSON payload            │
│ (big-endian)   │    (serialized message)    │
└────────────────┴────────────────────────────┘
```

**Why JSON?**
- Human-readable for debugging
- Easy to implement in any language
- Flexible schema evolution

**Why TCP?**
- Reliable delivery (we need ordered actions)
- Built-in flow control
- Simple, battle-tested

**Why not MessagePack/JSON?**
- MessagePack is not a wire protocol
- I wanted to try implementing my own wire protocol
- I figured prepending lengths was safer than expecting well formed json
- I'm probably dumb

---

# Lamport Clocks: Logical Time

Each peer has a **Lamport clock** (counter):

```rust
// LocalSettings table
clock: usize  // Current logical timestamp
```

**Rules:**
1. Before any local event: `clock += 1`
2. When sending message: include current `clock`
3. When receiving message: `clock = max(local_clock, received_clock) + 1`

**Result:** Total ordering of all events across all peers!

```
Peer A: event₁ [clock=1] → event₂ [clock=2]
Peer B:                   event₃ [clock=1]
After sync:
  event₁ [1] → event₂ [2] → event₃ [3]
  (B's clock becomes max(1,2)+1 = 3)
```

---

# Sync Handshake Phase

```
Client                              Server
  │                                   │
  │  PeerIdentificationHandshake      │
  ├──────────────────────────────────>│
  │  {                                │
  │    local_peer_id: UUID,           │
  │    local_server_address: addr     │
  │  }                                │
  │                                   │
  │  PeerIdentificationHandshakeResp  │
  │<──────────────────────────────────┤
  │  {                                │
  │    peer_id: UUID                  │
  │  }                                │
  │                                   │
 [Both peers now know each other]
 [Bidirectional peer relationship stored]
```

After handshake, peers can sync data!

---

# Sync Request/Response

```
Client                              Server
  │                                   │
  │  SyncRequest                      │
  ├──────────────────────────────────>│
  │  {                                │
  │    local_peer_id: UUID,           │
  │    last_sync_clock: 42            │
  │  }                                │
  │                                   │
  │      [Server queries Mutations    │
  │       WHERE clock > 42]           │
  │                                   │
  │  SyncResponse                     │
  │<──────────────────────────────────┤
  │  {                                │
  │    clock: 57,                     │
  │    changes: [Action, Action, ...] │
  │  }                                │
  │                                   │
```

Client receives all actions since last sync!

---

# The Sync Dance: Algorithm

**Problem:** Both peers have local changes since last sync

**Solution:** Rebase local changes on top of remote

```
Step 1: Request changes since last_sync_clock
Step 2: Receive remote changes + remote's current clock
Step 3: Undo all local changes made since last sync
        (pop from Mutations, apply inverses)
Step 4: Apply remote changes in order
        (ensure clock ordering)
Step 5: Reapply local changes on top
        (replay from temporary storage)
Step 6: Update peer's last_sync_clock in local DB
```

From `libshizen/src/sync.rs:113-146`

---

# Sync Dance Visualization

```
Initial state (both synced at clock=10):
  Peer A: ───[10]───
  Peer B: ───[10]───

Both make local changes:
  Peer A: ───[10]──[A1:11]──[A2:12]───
  Peer B: ───[10]──[B1:11]──[B2:12]───

A initiates sync with B:
  1. A undoes A1, A2  →  ───[10]───
  2. A applies B1, B2 →  ───[10]──[B1:11]──[B2:12]───
  3. A reapplies A1, A2 → ───[10]──[B1:11]──[B2:12]──[A1:13]──[A2:14]───
     (clocks rebased!)

Result: A has all changes in causal order!
```

**Note:** This is "last-write-wins" for now (local changes win after rebase)

This is ***A lot*** like git rebase.

---

# SyncServer: Background Thread

From `libshizen/src/sync.rs`:

```rust
pub struct SyncServer {
    handle: Option<JoinHandle<()>>,
    shutdown_tx: Sender<()>,
}

impl SyncServer {
    pub fn start(storage: Arc<Mutex<RusqliteStorage>>) -> Self {
        let (shutdown_tx, shutdown_rx) = channel();
        let handle = thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:1701").unwrap();
            listener.set_nonblocking(true).unwrap();
            // Accept connections until shutdown signal
            loop {
                if shutdown_rx.try_recv().is_ok() { break; }
                // ... handle connections ...
            }
        });
        SyncServer { handle: Some(handle), shutdown_tx }
    }
}
```

**Graceful shutdown:** Channel-based signaling, thread join

---

# SyncConnection: Client-Side

```rust
pub struct SyncConnection {
    stream: TcpStream,
}

impl SyncConnection {
    pub fn connect(addr: &str) -> ShizenResult<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(SyncConnection { stream })
    }

    pub fn handshake(&mut self, local_peer_id: PeerId,
                     local_addr: Option<SocketAddr>) -> ShizenResult<PeerId> {
        let request = SyncRequest::PeerIdentificationHandshake { ... };
        self.send_message(&request)?;
        let response = self.receive_message()?;
        // ... extract peer ID ...
    }

    pub fn sync(&mut self, storage: &mut RusqliteStorage,
                peer_id: PeerId) -> ShizenResult<()> {
        // The sync dance!
    }
}
```

---

# Conflict Resolution (Current)

**Current approach:** "Last-write-wins" (local bias)

When both peers edit the same note:
```
Peer A: UpdateTitle(note1, "Fix bug")    [clock=11]
Peer B: UpdateTitle(note1, "New feature") [clock=11]
```

After A syncs with B:
```
1. A undoes local change
2. A applies B's change → note1.title = "New feature"
3. A reapplies own change → note1.title = "Fix bug"
```

**Result:** A's change wins (because it's reapplied last)

**TODO:** Better conflict detection and resolution!

---

# CRDT-Inspired Design

**Not a full CRDT, but inspired by CRDT principles:**

✅ **What Shizen borrows from CRDTs:**
- Operation-based replication (Op-based CRDT)
- Causal ordering (Lamport clocks)
- Commutative operations (most actions commute)
- Eventual consistency goal

❌ **What Shizen doesn't have (yet):**
- True commutativity for all operations
- Automatic conflict-free merges
- Vector clocks for full causality tracking

**Philosophy:** "Eventually consistent is eventually correct... eventually!" 😅

---

# Future Conflict Handling

**Planned improvements:**

1. **Detect conflicts:** Track which note properties changed
1. **Merge strategies:**
   - Last-write-wins for simple fields
   - Three-way merge for descriptions (like git)
   - User prompt for irreconcilable conflicts
1. **Conflict markers:**
1. **Vector clocks** instead of Lamport clocks (better causality)

---

# Future Work: UI Development

**Currently:** Library-only, no GUI/TUI

**Planned UIs:**

1. **Tauri/Dioxus Desktop GUI**
   - Native desktop app
   - Dioxus (Rust) + Tauri (native webview)
   - Or: egui, iced, other Rust UI frameworks

2. **Ratatui TUI**
   - Terminal-based interface
   - Keyboard-driven (vim-like?)
   - Perfect for remote/SSH usage

3. **Web UI (WASM)**
   - Run in browser via WebAssembly
   - SQLite WASM with WASI-VFS
   - Local storage, optional sync

**Goal:** Multiple UIs, one libshizen core!

---

# Future Work: Sync Improvements

**TODO: Better merge conflict handling**
- Detect when same note modified by multiple peers
- Three-way merge for text fields
- User-friendly conflict resolution UI

**TODO: Concurrent multi-peer sync**
- Currently syncs one peer at a time
- Could spawn threads to sync multiple peers concurrently

**TODO: Notify peers to sync back**
- After receiving changes, notify peer to pull back
- Implement bidirectional sync triggers

**TODO: Vector clocks**
- Better causality tracking than Lamport clocks
- Can detect concurrent vs. sequential updates

---

# Future Work: Web Port (WASM)

**Goal:** Run Shizen in the browser!

**Plan:**
- Compile Rust to WebAssembly
- Use SQLite WASM (via WASI-VFS)
- Store data in browser local storage
- P2P sync via WebRTC or WebSocket

**Challenges:**
- TCP not available in browser (need WebSocket/WebRTC)
- File system abstraction (WASI-VFS)
- Browser storage limits

**Already prepared:** `rusqlite` dependency has `wasm32-wasi-vfs` feature flag!

---

# Future Work: Systemd Service

**Goal:** Run Shizen as a background daemon

```ini
[Unit]
Description=Shizen P2P Sync Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/shizen-server
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

**Use case:**
- Always-on sync server on home server/VPS
- Other devices sync to this "hub" peer
- Could run on Raspberry Pi!

---

# Future Work: Other TODOs

From the codebase:

- **CLI improvements:** Sort ordering support, better UX
- **Redo bug:** Clear redo table on non-undo/redo operations
- **Performance:** Benchmarking, profiling, optimizations
- **Testing:** More edge cases, property-based tests
- **Documentation:** API docs, user guide, examples
- **Cycle detection:** Prevent circular dependencies in DAG
- **Configurability:** Custom ports, storage paths, sync strategies

**Lots of room for contributions!**

---

# Current Usage: Library-Only

**Shizen is currently a library** (not a standalone app)

**Integration example:**
```rust
use libshizen::{RusqliteStorage, TodoStorage, NoteId};

fn main() {
    let mut storage = RusqliteStorage::new("~/todos.db").unwrap();

    let id = NoteId::new();
    storage.create_note(id, "Build shizen".into(), None).unwrap();
    storage.set_completed(id, true).unwrap();

    let notes = storage.all_notes().unwrap();
    for note in notes {
        println!("{}: {}", note.id, note.title);
    }
}
```

**CLI exists but is minimal** (basic CRUD only)

---

# Getting Started

**Prerequisites:**
- Rust toolchain (rustup)
- SQLite (bundled with rusqlite)

**Build libshizen:**
```bash
git clone https://github.com/TODO/shizen  # TODO: Add your repo
cd shizen
cargo build --release -p libshizen
cargo test -p libshizen
```

**Use in your project:**
```toml
[dependencies]
libshizen = { git = "https://github.com/brandonpollack23/shizen", commit = "..." }

not on crates.io yet

---

# Architecture Philosophy

**Design principles:**

1. **Modular:** Clean separation of storage, sync, UI
2. **Trait-based:** Program to interfaces
3. **Type-safe:** Leverage Rust's type system
4. **Local-first:** Data sovereignty matters
5. **Simple:** Prefer simple solutions over complex ones
6. **Tested:** Comprehensive unit tests (especially in storage layer)

**Inspiration:**
- CRDTs (Conflict-Free Replicated Data Types)
- Git (operation log, merge strategies)
- SQLite (embedded, reliable, simple)

---

# Contributing

**Shizen is open source!**

- **Repo:** github.com/brandonpollack23/shizen

**Areas for contribution:**
- UI development such as a GUI, TUI (Ratatui?), etc.
- More backends
- web version? (Good luck getting sqlite to run in browser in a way that meshes
  well).
- Testing & bug fixes
- Documentation

**First-time contributors welcome!**

---

# Summary

**Shizen** = Local-first P2P hierarchical todo management

**Key Takeaways:**
- ✅ Dependency based tasks
- ✅ No central server (true P2P)
- ✅ Operation-based replication (action log)
- ✅ Lamport clocks (distributed ordering)
- ✅ LexoRank (efficient custom ordering)
- ✅ Perfect undo/redo (mutation history)
- ✅ Rust safety + performance

**Status:** Library-complete, CLI working, other UIs TODO

---

# Q&A

**Questions?**

---

# Thank You!

Please reach out on LinkedIn or Email for hiring/consulting work or if you'd
just like to connect.

LinkedIn:

█████████████████████████████████████
█████████████████████████████████████
████ ▄▄▄▄▄ █▀▀ █▄█▄▀  ▄▄ █ ▄▄▄▄▄ ████
████ █   █ █▀██ ▀▀▀▄▄▀█▄██ █   █ ████
████ █▄▄▄█ █▀▄██▄ ▀▀█▄▄▄▄█ █▄▄▄█ ████
████▄▄▄▄▄▄▄█▄▀▄█ █ █ █▄▀ █▄▄▄▄▄▄▄████
████▄▄▄▄▄▀▄▄  ▀▀▀ ▄█▀   █ ▀ ▀▄█▄▀████
████ █▀▄▄█▄▀▄ ▄█ █▄▄ ▄█▀▀▀▄▀█▀█▀█████
████   ▄▀▄▄██▄█▀  ▄█▀ ▀ ▀▀▀█▀▄▄█▀████
████ ▀▄▄▄ ▄  ▀█▀ █▄ ▀█▄█▄▄▄ ▀▄▄▀█████
████ █▄▀▄▄▄▄▄██▀▄▄▄▄▀ ▀ ▀▀▀ ▀▄ █▀████
████ █▀ █▄▄▄▀▄██ █▀▄▀▄▀█▀▀  ▀█▄▀█████
████▄█▄▄▄▄▄▄  ▀ ▀▄▄█▀  █ ▄▄▄ ▀   ████
████ ▄▄▄▄▄ █▄▀ ██▄▀ ▄▄█  █▄█ ▄▄█▀████
████ █   █ █  ▀▀▀ ▄▄▀▄▀▀▄▄▄ ▄▀ ▀ ████
████ █▄▄▄█ █ █ ██▄▀▄█▄ ▄▀   ▄ ▄ █████
████▄▄▄▄▄▄▄█▄█▄▄▄▄█▄▄▄█▄██▄▄▄▄▄██████
█████████████████████████████████████
█████████████████████████████████████

Email:

█████████████████████████████████████
█████████████████████████████████████
████ ▄▄▄▄▄ █ ▄▄ █▄▄ █ ██▄█ ▄▄▄▄▄ ████
████ █   █ ██▄█▀▀▄█▀▀▄▀█ █ █   █ ████
████ █▄▄▄█ █ ▀▀▄ ▀▀▄ ▄▄█ █ █▄▄▄█ ████
████▄▄▄▄▄▄▄█ ▀▄█▄▀▄█▄█▄▀▄█▄▄▄▄▄▄▄████
████▄▄ ▀▀ ▄▀ ▄█▄ ▄█▄ ██▀ ▄  ▄▀▀▄ ████
█████▀▄▀▀▄▄▀▀▄ ▀ ▄ ▀ █▀ █▀▀ ▄ ▄▀▄████
████ █▄ █ ▄█▄  ██  █▄▀█  █  █▀█▄▀████
████  ▀ █▄▄█▀█ ▄█▀█▄▀▄▀▄██▀ ▀ ▄▀▄████
████▄ ▄  ▀▄ █ █▄ ▄█▀ ██  ▀  █ ▀▄ ████
████▄▄█ ▀▄▄ █▄▀▀ ▄  ▀▄ ▄ ▄▄█▄██▀▄████
████▄▄▄▄▄▄▄█ ▀███  ▀ ▀██ ▄▄▄ █▀█▀████
████ ▄▄▄▄▄ █▀▀▀▄█▀█ █▄ ▀ █▄█  █▀▄████
████ █   █ ██▄ ▄ ▄█  ▀█▄▄ ▄▄ ▄█▄ ████
████ █▄▄▄█ █ █▄▀ ▄   ▄██▀ ▀█  █▄▄████
████▄▄▄▄▄▄▄█▄▄███▄▄█▄█▄█▄██▄▄███▄████
█████████████████████████████████████
█████████████████████████████████████

Please consider sponsoring TokyoRust.org!

---
