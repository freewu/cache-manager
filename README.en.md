# Cache Manager

> 💡 **This project was developed entirely using [pi agent](https://github.com/earendil-works/pi) + DeepSeek V4 Flash**

A desktop cache management tool built with **Tauri 2 + Rust + Vue 3**, supporting both **Redis** and **Memcached**. Redis supports **Standalone / Master-Slave / Sentinel / Cluster** deployment modes.

![tech](https://img.shields.io/badge/Tauri-2-24c8db) ![tech](https://img.shields.io/badge/Rust-fred-red) ![tech](https://img.shields.io/badge/Vue-3-42b883)

[中文](README.md) | **English**

## Features

### Connection Management
- Supports **Redis** (Standalone / Master-Slave / Sentinel / Cluster) and **Memcached**
- ACL username/password authentication, TLS/SSL, connect timeout configuration
- Connection config persisted locally (`%APPDATA%/com.cachemanager.app/connections.json`)
- One-click connection test, parallel management of multiple connections
- **Import / Export connections**: export to JSON for cross-machine migration; automatically skips duplicates with the same host:port on import
- Quick-connect saved connections from the **system tray** (click to connect and open)

### Key Space Browser
- Redis: `SCAN` cursor pagination, pattern search and type filter (String/Hash/List/Set/ZSet/Stream); automatic cross-node scan merge in cluster mode
- Memcached: enumerates all keys via `stats items` + `stats cachedump` (falls back to `lru_crawler metadump` on older servers)
- **Local fuzzy search**: Memcached keys are filtered locally — type any substring for instant fuzzy matching (`*` / `?` wildcards supported), without repeated server requests
- Key info: type, TTL, length, memory usage
- Master-Slave mode supports reading from **replicas** (read/write split), switchable in the UI
- Hover a key to **delete it** (with confirmation dialog)

### Value View & Edit
- **String**: text/Base64 encoding toggle, TTL support; JSON values auto-formatted with syntax highlighting
- **Hash**: field list, add/delete fields (large hashes paginate via `HSCAN`)
- **List**: element list, `LPUSH`/`RPUSH` append, delete by value
- **Set**: member list, add/delete
- **ZSet**: members + scores, add/delete
- **Stream**: entries, `XADD`/`XDEL`
- Binary-safe: non-UTF-8 content auto-displayed as Base64
- Create new keys of all six types; delete the current key directly from the value editor

### Command Console
- Run any Redis / Memcached command (quoted-argument parsing supported)
- In Memcached mode the `set` command is auto-completed (e.g. `set key value [exptime]` → full protocol)
- Responses formatted as JSON, scalar values shown as plain text
- Command history (↑/↓), execution time stats

### Server Management
- **INFO**: sectioned display (server/memory/clients/replication/keyspace…)
- **CONFIG**: query and modify configuration
- **CLIENT LIST**: connected clients
- **SLOWLOG**: slow query log
- **Topology view**: master/slave, cluster, sentinel node status overview

### Real-time Monitoring
- **Pub/Sub**: subscribe to channels/patterns, real-time message push, publish support
- **MONITOR**: stream every command received by the server (standalone/master-slave modes)

### System Tray & Settings
- Minimize to **system tray** on window close (keeps running in the background)
- Tray menu: show main window, **quick-connect saved connections** (with Redis/Memcached type icons), quit
- Dedicated settings page (reachable from tray): **Light / Dark / Follow system** themes, close behavior, **default export directory**
- Single instance (launching again focuses the existing window)

## Screenshots

| Redis data browsing | Memcached data browsing |
| --- | --- |
| ![Redis data browsing](docs/images/redis-date.png) | ![Memcached data browsing](docs/images/memcache-data.png) |

| Settings page | System tray |
| --- | --- |
| ![Settings page](docs/images/settingpng.png) | ![System tray](docs/images/tray.png) |

## Architecture

```
┌─────────────────────────────┐
│   Vue 3 + Naive UI frontend  │  Tauri IPC (invoke / Channel)
└──────────────┬──────────────┘
               │
┌──────────────▼──────────────┐
│   Rust backend (tauri commands) │
│  ┌────────────────────────┐  │
│  │ RedisManager (pool mgmt) │
│  │  - Single / MasterSlave│  │
│  │  - Sentinel / Cluster  │  │
│  │ keys / console / server │  │
│  │ pubsub / monitor        │  │
│  └───────────┬────────────┘  │
│  ┌───────────▼────────────┐  │
│  │ MemcachedManager       │  │
│  │ (self-implemented text │  │
│  │  protocol, tokio)      │  │
│  └───────────┬────────────┘  │
└──────────────┼──────────────┘
        ┌──────▼──────┐
        │  fred (Redis │
        │  client)     │
        └─────────────┘
```

- **Redis client**: [fred](https://github.com/redis-rs/fred) v10 — native standalone, cluster (auto-discovery / slot routing / MOVED redirect), sentinel (master discovery / failover), replication
- **Memcached**: self-implemented text protocol (`memcachedx.rs`, tokio `TcpStream`, zero third-party dependencies)
- **Protocol**: RESP3 (auto-fallback to RESP2), binary-safe
- **Connection pool**: dedicated `RedisPool` per connection; replicas get their own pool in master-slave mode

## Quick Start

### Prerequisites
- [Node.js](https://nodejs.org) ≥ 18
- [Rust](https://www.rust-lang.org) ≥ 1.77 (with platform toolchain)
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
npm install
npm run tauri dev
```

### Using just (recommended)

[just](https://github.com/casey/just) is used to unify common dev commands. After installing just:

```bash
just dev       # dev run (hot reload)
just build     # release build (portable single-file exe)
just release   # generate release artifacts (exe + source tarball + hashes + git tag)
just typecheck # frontend type check
just test      # backend unit tests
```

Run `just --list` to see all recipes (build / tests / icon generation, etc.).

> **Cross-platform**: the justfile works on macOS / Linux / Windows. On Windows, install [Git for Windows](https://git-scm.com) and add `Git\bin` to PATH (provides `sh`, which just uses to run commands); macOS / Linux work out of the box.

### Build (portable single file)

```bash
npm run tauri build
```

The output is a **portable single file** `src-tauri/target/release/cache-manager.exe` (copied to `release/CacheManager.exe`). Double-click to run — no installation or extra dependencies (only requires the WebView2 runtime bundled with Windows 10/11).

> Version: current version is **v0.1.1**. The version number is maintained in three places (keep them in sync): `Cargo.toml` / `tauri.conf.json` / `package.json`.
> After building, right-click → Properties → Details on the exe shows the file/product version (written by tauri-build from `Cargo.toml` into the VERSIONINFO resource).

### Release artifacts (`just release`)

```bash
just build    # build the exe first
just release  # generate release artifacts
```

Generated into `release/`:

```
CacheManager-0.1.1.exe        # versioned portable exe
CacheManager-0.1.1.exe.md5    # MD5 checksum of the exe
CacheManager-0.1.1.exe.sha1   # SHA1 checksum of the exe
source-0.1.1.tar.gz           # source tarball (excludes node_modules/target/.git/release/dist, etc.)
source-0.1.1.tar.gz.md5       # MD5 of the tarball
source-0.1.1.tar.gz.sha1      # SHA1 of the tarball
```

It also creates and pushes a git tag matching the version (`v0.1.1`, skipped automatically if it already exists).

### Tests

Backend smoke tests against a real Redis (default `127.0.0.1:6379`):

```bash
# Standalone
cargo test --test smoke
# Master-Slave (master:6380, replica:6381)
cargo test --test master_slave
# Sentinel (sentinel:26379, master:6380)
cargo test --test sentinel
# Cluster (3 nodes 7000-7002)
cargo test --test cluster
# URL building unit tests
cargo test --test url_build
```

## Project Structure

```
├── src/                        # Vue 3 frontend
│   ├── api/                    # Tauri invoke wrappers
│   ├── components/             # ConnectionForm / ValueEditor
│   ├── views/                  # Connections / Explorer / Console / ServerInfo / Monitor / Settings
│   ├── store/                  # Pinia connection state
│   ├── router/                 # Router
│   └── types.ts                # Types matching backend DTOs
├── src-tauri/
│   ├── src/
│   │   ├── commands.rs         # Tauri command layer (thin wrapper, mode routing)
│   │   ├── redisx/
│   │   │   ├── mod.rs          # Connection manager, URL building, mode routing
│   │   │   ├── keys.rs         # Key scan / value read-write
│   │   │   ├── console.rs      # Arbitrary command execution
│   │   │   ├── server.rs       # INFO/CONFIG/CLIENTS/SLOWLOG/topology
│   │   │   └── stream.rs       # Pub/Sub + MONITOR
│   │   ├── memcachedx.rs       # Memcached text protocol (zero deps)
│   │   ├── model.rs            # Connection config models
│   │   ├── store.rs            # Config/settings persistence
│   │   ├── tray.rs             # System tray (quick connect + icons)
│   │   ├── single_instance.rs  # Single-instance port lock
│   │   └── error.rs            # Unified error type
│   └── tests/                  # Smoke tests (standalone/master-slave/sentinel/cluster)
└── scripts/gen_icon.py         # Icon generation script
```

## Mode Overview

| Mode | Connection | Supported Operations |
|------|-----------|---------------------|
| Standalone | Direct | All features |
| Master-Slave | Master read/write; replica browse/read (UI switch) | All; replicas read-only |
| Sentinel | Master discovered via Sentinel, failover support | All; topology shows sentinel/master-slave status |
| Cluster | Seed node auto-discovery, slot routing, MOVED redirect | All; cross-node SCAN |
| Memcached | Direct (text protocol) | Key browse/fuzzy search/value edit/console/delete/TTL |

## Notes
- `MONITOR` works only in standalone/master-slave modes (Redis limitation)
- `SELECT` is unsupported in cluster mode (Redis Cluster only has db0)
- Large keys (>5000 elements) paginate automatically with a truncation marker
- Memcached key enumeration relies on `stats items` / `stats cachedump` (enabled by default on servers); falls back to `lru_crawler metadump` when disabled
- Connection import deduplicates by **host:port**; duplicate configs are skipped automatically
