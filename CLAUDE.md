# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Orbit is a local development environment manager for Windows, built with **Tauri 2** (Rust backend + React frontend). It provides 1-click installation and management of 17+ services: Nginx, Apache, PHP (multi-version), MariaDB, PostgreSQL, MongoDB, Redis, Meilisearch, Node.js, Python, Bun, Deno, Go, Rust, Mailpit, Composer, and Ngrok. Also integrates AI tools (Claude Code, Gemini CLI) and SSH/SFTP/FTP deploy.

## Build & Dev Commands

```bash
bun install              # Install frontend dependencies
bun dev                  # Vite dev server only (frontend)
bun tauri dev            # Full Tauri dev window (frontend + Rust backend)
bun tauri build          # Production build → NSIS installer at core/target/release/bundle/
bun run build            # Frontend-only build (tsc + Vite)
bun run lint             # ESLint
```

Rust backend is compiled automatically by `bun tauri dev` / `bun tauri build`. Manual Rust commands from `core/`:
```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo clippy             # Rust linter
cargo build --features mcp --bin orbit-mcp    # MCP server binary
```

## Architecture

### Frontend (`app/`)
- **React 19** + **TypeScript** + **TailwindCSS v4** + **Vite**
- Entry: `app/main.tsx` → providers stack → `App.tsx` (sidebar + tab views)
- State: React Context (`AppContext` for services/settings/toasts, `ThemeContext` for dark/light)
- Persistent storage via `@tauri-apps/plugin-store`
- IPC calls via `invoke()` from `@tauri-apps/api/core`, all wrapped in `app/lib/api/` (modular: services, sites, database, config, tools, ai-tools, deploy)
- Terminal: xterm.js 5 with PTY backend from Rust
- Custom titlebar (window decorations disabled in Tauri config)

### Backend (`core/src/`)
- **Rust** with Tauri 2 plugin ecosystem
- Entry: `main.rs` → `lib.rs` (plugin init, handler registration, system tray setup)
- `commands/` — Tauri invoke handlers (~27 modules): service management, installation, sites, SSL, logs, database ops, PHP config, PATH management, AI tools, deploy, etc.
- `services/` — Business logic (~30 modules): process lifecycle (`process.rs`), Nginx/Apache config generation, PHP registry, download utilities, PTY terminal, templates, validation, AI tool managers, deploy store/engine
- `cli.rs` — Separate `orbit-cli` binary (clap + colored) with commands: start, stop, restart, status, list, install, uninstall, logs, config, scan, open, trust-ssl. Supports service aliases (pg, mongo, maria, node). Does NOT link against the Tauri lib crate.
- `mcp.rs` — Separate `orbit-mcp` binary (MCP server over stdio). Exposes 72+ tools across 12 domains: services, sites, databases (MariaDB + PostgreSQL), logs, PHP config, SSL, Redis, Composer, Mailpit, config files, hosts, service install/uninstall, and deploy (list connections, test, SSH execute, sync, status). No async runtime, no Tauri dependency. Build with `cargo build --features mcp --bin orbit-mcp`.
- ServiceManager is a shared `State<ServiceManager>` across all handlers

### IPC Pattern
Frontend `invoke("command_name", { args })` → Rust `#[tauri::command] fn command_name(...)` → Returns `Result<T, String>` serialized as JSON.

### Service Download Registry
Lives in a sibling repo: `orbit-libraries/` (https://github.com/alinsgit/orbit-libraries)
- `scripts/fetch-versions.js` scrapes official sources daily for latest versions
- Outputs `dist/libraries.json` with download URLs per platform
- Consumed by `core/src/services/registry.rs` which fetches from GitHub raw URL
- Has embedded fallback registry compiled into the binary for offline mode
- GitHub Actions auto-updates daily at 00:00 UTC

### Theme System
CSS variables in `app/index.css` define dark/light palettes. Primary color: emerald (#10b981). Tailwind maps these via `@theme` directive.

### Key Patterns
- Window hides to system tray on close (not destroyed)
- Port conflicts detected via `TcpListener` binding
- PHP multi-version support with port mapping in `PhpRegistry`
- Site templates: HTTP, HTTPS, Static, Laravel, WordPress, LiteCart + JS framework reverse proxy (Next.js, Astro, Nuxt, Vue)
- SSL via OpenSSL self-signed certificate generation
- Hosts file editing for local domain resolution

### AI Tools Integration
- **Claude Code** and **Gemini CLI** as first-class AI tools (install/update/uninstall via npm)
- AI Tool View: multi-session terminal with project selector + collapsible info panel
- Auto MCP config: on Claude Code install, adds `orbit-mcp` to `~/.claude.json`
- Context generation: creates `.claude/orbit-context.md` with project info, services, git status
- Sidebar shows Claude Code / Gemini CLI icons when installed

### Deploy System
- SSH/SFTP/FTP deploy connections per site (connection CRUD in site cards)
- Credentials stored in OS keyring via `keyring-rs`, metadata in `config/deploy-connections.json`
- Diff-based file sync using blake3 hashing — only changed files uploaded
- Deploy manifests stored in `config/deploy-manifests/{domain}/{conn}.json`
- `.deployignore` support via `ignore` crate (falls back to `.gitignore`)
- Real-time progress via Tauri events (`deploy-progress`)
- Deploy lock prevents concurrent deploys to same connection
- Smart first sync: when no manifest exists, scans remote file sizes via SSH and skips identical files

## Release Flow

```bash
# 1. Version bump (updates tauri.conf.json, Cargo.toml, package.json)
bun run bump 1.2.0

# 2. Update website hero badge in docs/index.html

# 3. Commit, tag, push
git add -A && git commit -m "bump: v1.2.0"
git tag -a v1.2.0 -m "Orbit v1.2.0"
git push && git push origin v1.2.0
```

**Critical:** Always run `bun run bump` before tagging. The updater reads version from `tauri.conf.json` — if it matches `latest.json`, users won't see the update.

CI workflow "Build & Release" triggers on tag push → builds macOS/Linux/Windows → uploads installers + `latest.json` to GitHub Release.

## Git Workflow

- **main** — single branch, all development and releases
- Tags trigger CI builds (`v*` pattern)

## Code Standards

- **Indentation**: 2 spaces
- **File naming**: kebab-case
- **Variables**: camelCase
- **Components**: PascalCase
- **Language**: TypeScript (frontend), Rust (backend)
- **Formatting**: Prettier

## File Layout

```
app/                    # React frontend
  components/           # UI components (ServiceManager, SitesManager, Terminal, LogViewer, etc.)
  lib/                  # Contexts, API wrappers, store, utilities
core/                   # Tauri/Rust backend
  src/commands/         # Invoke handlers
  src/services/         # Business logic modules
  tauri.conf.json       # Tauri app config (window, CSP, bundler, updater)
docs/                   # GitHub Pages landing site
```
