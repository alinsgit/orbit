# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Orbit is a local development environment manager for Windows, built with **Tauri 2** (Rust backend + React frontend). It provides 1-click installation and management of services like Nginx, PHP, MariaDB, PostgreSQL, MongoDB, Redis, Node.js, Python, Bun, and more.

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
```

## Architecture

### Frontend (`app/`)
- **React 19** + **TypeScript** + **TailwindCSS v4** + **Vite**
- Entry: `app/main.tsx` → providers stack → `App.tsx` (sidebar + tab views)
- State: React Context (`AppContext` for services/settings/toasts, `ThemeContext` for dark/light)
- Persistent storage via `@tauri-apps/plugin-store`
- IPC calls via `invoke()` from `@tauri-apps/api/core`, all wrapped in `app/lib/api.ts`
- Terminal: xterm.js 5 with PTY backend from Rust
- Custom titlebar (window decorations disabled in Tauri config)

### Backend (`core/src/`)
- **Rust** with Tauri 2 plugin ecosystem
- Entry: `main.rs` → `lib.rs` (plugin init, handler registration, system tray setup)
- `commands/` — Tauri invoke handlers (~25 modules): service management, installation, sites, SSL, logs, database ops, PHP config, PATH management, etc.
- `services/` — Business logic (~27 modules): process lifecycle (`process.rs`), Nginx/Apache config generation, PHP registry, download utilities, PTY terminal, templates, validation
- `cli.rs` — Separate `orbit-cli` binary for CLI access
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
- Site templates: HTTP, Laravel, WordPress, LiteCart, Static, Next.js, Astro, Nuxt, Vue
- SSL via OpenSSL self-signed certificate generation
- Hosts file editing for local domain resolution

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
