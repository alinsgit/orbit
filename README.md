# Orbit

AI-ready local development environment for **Windows, Linux and macOS**. 17 services, MCP integration, CLI — all from a single interface.

## Features

- **Service Management** — Install, start, stop and configure 17 services with one click
- **MCP Integration** — 72+ tools across 12 domains for Claude Code, Cursor and Windsurf
- **Orbit CLI** — Full command-line interface: start, stop, install, status, db, hosts and more
- **AI Tools** — Launch Claude Code and Gemini CLI directly from Orbit with project context and MCP auto-config
- **Site Management** — Local sites with custom domains (.test), Nginx/Apache vhosts, templates
- **Deploy** — SSH/SFTP/FTP deploy with diff-based sync, global connections and per-site targets
- **Local SSL** — Self-signed certificates via mkcert, one-click HTTPS
- **Database Tools** — MariaDB + PostgreSQL management, Adminer, phpMyAdmin
- **Multi-version PHP** — Run multiple PHP versions side by side with per-site selection
- **Log Viewer** — Real-time log monitoring with filtering by service and level
- **Integrated Terminal** — Built-in terminal with PTY support
- **PATH Management** — Service directories added to system PATH automatically on install
- **Autostart** — Per-service autostart on app launch
- **Blueprints** — Scaffold Laravel, Next.js, Astro, WordPress and more with one click

## Supported Services

Nginx, Apache, PHP (multi-version), MariaDB, PostgreSQL, MongoDB, Redis, Meilisearch, Node.js, Python, Bun, Deno, Go, Rust, Composer, Mailpit, Ngrok

## Platform Support

| Feature | Windows | Linux | macOS (Apple Silicon) |
|---------|---------|-------|----------------------|
| PHP (pre-built binaries) | ✅ | ✅ | ✅ |
| MariaDB, MongoDB, PostgreSQL | ✅ | ✅ | ✅ |
| Node.js, Bun, Go, Deno | ✅ | ✅ | ✅ |
| Nginx | ✅ | ✅ | ✅ |
| Rust (via rustup) | ✅ | ✅ | ✅ |
| Mailpit, Meilisearch, Ngrok | ✅ | ✅ | ✅ |
| Composer | ✅ | ✅ | ✅ |
| Python | ✅ | ✅ | ✅ |
| Redis | ✅ | — | — |
| Apache | ✅ | — | — |
| PATH management | Registry | `~/.bashrc` / `~/.zshrc` | `~/.zshrc` / `~/.bash_profile` |
| MCP + CLI | ✅ | ✅ | ✅ |

> Redis and Apache are Windows-only installs. On Linux/macOS, native package managers (apt, brew) are preferred.

## MCP Integration

Orbit exposes 72+ tools via the Model Context Protocol for Claude Code, Cursor, Antigravity, Windsurf and any MCP-compatible client.

After installing MCP from the Orbit app, add the following to your AI tool's config:

```json
{
  "mcpServers": {
    "orbit": {
      "command": "orbit-mcp"
    }
  }
}
```

> Orbit automatically adds `orbit-mcp` to your system PATH on install — no full path needed.

### MCP Domains

Services, Sites, MariaDB, PostgreSQL, MongoDB, Redis, PHP Config, SSL, Logs, Composer, Mailpit, Config/Hosts, Deploy

## AI Tools

Orbit integrates Claude Code and Gemini CLI as first-class AI tools:

- Install/update/uninstall from the AI Tools tab
- Launch with project context — Orbit writes `.claude/orbit-context.md` with active services, deploy targets, git info
- Auto-configures `orbit-mcp` in Claude Code's MCP config on install
- Multi-session terminal with project selector

## Deploy

SSH/SFTP/FTP deploy with global server connections and per-site targets:

- Diff-based file sync using blake3 hashing — only changed files uploaded
- Credentials stored in OS keyring
- `.deployignore` support (falls back to `.gitignore`)
- Real-time progress tracking
- Deploy manifests for incremental syncs

## CLI

```bash
orbit-cli status              # Show all services and their status
orbit-cli start nginx         # Start a service
orbit-cli stop mariadb        # Stop a service
orbit-cli install redis       # Install a service
orbit-cli logs nginx          # View service logs
orbit-cli db list             # List MariaDB databases
orbit-cli hosts list          # Show hosts file entries
orbit-cli config php 8.4      # Edit PHP config
```

Supports aliases: `pg` for postgresql, `maria` for mariadb, `mongo` for mongodb, `node` for nodejs.

## Getting Started

### Prerequisites

- [Bun](https://bun.sh) (package manager)
- [Rust](https://rustup.rs) (for Tauri backend)

### Development

```bash
bun install
bun tauri dev
```

### Build

```bash
bun tauri build
```

## Project Structure

```
app/     React + TypeScript frontend
core/    Rust (Tauri 2) backend
docs/    GitHub Pages landing site
```

## License

[MIT](LICENSE)
