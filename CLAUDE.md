# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Shizen is a hierarchical task/note management system with synchronization capabilities. It consists of:

- `libshizen`: Core library with storage, synchronization, and entity definitions
- `shizen-cli`: Command-line interface for managing tasks/notes
- `shizen-tauri`: Tauri-based UI (replacing previously removed Dioxus UI)

## Build and Run Commands

### Build the Project

```bash
# Build all workspace members
cargo build

# Build with release optimizations
cargo build --release

# Build a specific package
cargo build -p libshizen
cargo build -p shizen-cli
cargo build -p shizen-tauri
```

### Run the CLI

```bash
# Run the CLI
cargo run -p shizen-cli -- [ARGS]

# Common commands:
# Create a new database
cargo run -p shizen-cli -- create

# List all tasks (including blocked tasks)
cargo run -p shizen-cli -- list --show-blocked

# List unblocked tasks
cargo run -p shizen-cli -- list

# Add a new task
cargo run -p shizen-cli -- add "Task Title" --description "Task Description"

# Add a task with a parent
cargo run -p shizen-cli -- add "Subtask" --parent-id "<UUID>"

# Mark a task as complete
cargo run -p shizen-cli -- complete <UUID> true

# Mark a task as incomplete
cargo run -p shizen-cli -- complete <UUID> false

# Update a task
cargo run -p shizen-cli -- update --note-id <UUID> --title "New Title" --description "New Description"

# Remove a task
cargo run -p shizen-cli -- remove <UUID>

# Add a dependency (from blocks to)
cargo run -p shizen-cli -- add-dependency <BLOCKER_UUID> <BLOCKEE_UUID>

# Reorder tasks
cargo run -p shizen-cli -- reorder before <ID> <BEFORE>
cargo run -p shizen-cli -- reorder after <ID> <AFTER>

# Undo/Redo
cargo run -p shizen-cli -- undo
cargo run -p shizen-cli -- redo

# View history and redo queue
cargo run -p shizen-cli -- history
cargo run -p shizen-cli -- redo-queue

# Sync commands
cargo run -p shizen-cli -- serve --port 1701
cargo run -p shizen-cli -- peer add <SOCKET_ADDR> [LOCAL_SERVER_PORT]
cargo run -p shizen-cli -- peer list
cargo run -p shizen-cli -- peer remove <UUID>
cargo run -p shizen-cli -- sync [--peer <UUID>]
```

### Run the Tauri UI

```bash
# Navigate to the Tauri directory
cd shizen-tauri

# Install npm dependencies
npm install

# Start the development server
npm run tauri dev

# Build for production
npm run tauri build
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test -p libshizen

# Run a specific test
cargo test -p libshizen -- peer_sync_one_way
```

## Architecture

### Core Components

1. **Entity Model**
   - `Note`: Represents a task/note with hierarchical and dependency relationships
   - `NoteId`: UUID-based identifier for notes
   - `PeerId`: UUID-based identifier for sync peers
   - `Action`: Enumeration of all possible actions that can be performed on notes

2. **Storage System**
   - `TodoStorage`: Trait defining the storage interface
   - `RusqliteStorage`: SQLite implementation of the storage interface
   - Operations include CRUD for notes, dependencies, undo/redo, and peer synchronization

3. **Synchronization System**
   - `SyncServer`: Server implementation that listens for sync connections
   - `SyncConnection`: Client-side connection to a sync server
   - Includes peer discovery and bidirectional sync of changes

4. **CLI Interface**
   - Command-line interface for interacting with the system
   - Uses `clap` for argument parsing
   - Provides commands for all note management and sync operations

5. **Tauri UI**
   - Web-based user interface using Tauri, React, and TypeScript
   - Communicates with the Rust backend via Tauri commands

### Key Patterns

1. **Event-based Storage**
   - Actions are stored in a history that can be used for undo/redo
   - Synchronization relies on sharing action history between peers

2. **Peer-to-Peer Synchronization**
   - Nodes can act as both clients and servers
   - Changes are synchronized by exchanging action logs since a given clock value

3. **Hierarchical Note Structure**
   - Notes can have parent-child relationships
   - Notes can have blocking relationships (dependencies)

## Database Structure

The system uses SQLite with tables for:
- Notes (with hierarchical structure)
- Actions/events (for history and sync)
- Peers (for synchronization)
- Redo queue (for managing undo/redo state)

## Development Notes

### Current TODOs (From Source)

- CLI support for sort ordering
- Fix bug with redo table handling
- Complete Tauri UI implementation
- Tracing spans/benchmarks/profiling
- Systemd service server with CLI
- Merge conflict detection/handling
- Web port via SQLite WASM