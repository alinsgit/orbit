# Orbit

AI-ready local development environment for Windows. 16 services, MCP integration, CLI — all from a single interface.

## Features

- **Service Management** — Install, start, stop and configure 16 services with one click
- **MCP Integration** — 51 tools across 10 domains for Claude Code, Cursor and Windsurf
- **Orbit CLI** — Full command-line interface: start, stop, install, status, db, hosts and more
- **Site Management** — Local sites with custom domains (.test), Nginx/Apache vhosts, templates
- **Local SSL** — Self-signed certificates via mkcert, one-click HTTPS
- **Database Tools** — MariaDB + PostgreSQL management, Adminer, phpMyAdmin
- **Multi-version PHP** — Run multiple PHP versions side by side with per-site selection
- **Log Viewer** — Real-time log monitoring with filtering by service and level
- **Integrated Terminal** — Built-in terminal with PTY support
- **PATH Management** — Add/remove services from system PATH per service
- **Autostart** — Per-service autostart on app launch

## Supported Services

Nginx, Apache, PHP (multi-version), MariaDB, PostgreSQL, MongoDB, Redis, Node.js, Python, Bun, Deno, Go, Rust, Composer, Mailpit, Ngrok

## MCP Integration

Orbit exposes 51 tools via the Model Context Protocol, organized across 10 domains:

| Domain | Tools |
|--------|-------|
| Services | list, status, start, stop, restart, start all, stop all, install, uninstall |
| Sites | list, create, delete, get config, read/write vhost config |
| MariaDB | list databases, create, drop, list tables, describe, execute query, export, import |
| PostgreSQL | list databases, list tables, describe, execute query |
| Logs | list, read, clear |
| PHP Config | list extensions, toggle extension, get/set config |
| SSL | generate certificate, list certificates |
| Redis | execute command, server info |
| Composer | require, install, run script |
| Mailpit | list emails, get email, delete all |
| Config Files | read/write service config, read/write site config |
| Hosts | list, add, remove |
| System | system info, run orbit command |

### Configuration

Add to your AI tool's MCP config (Claude Code `~/.claude.json`, Cursor, Windsurf):

```json
{
  "mcpServers": {
    "orbit": {
      "command": "C:\\Users\\<USER>\\AppData\\Local\\com.orbit.dev\\bin\\mcp\\orbit-mcp.exe",
      "args": []
    }
  }
}
```

## CLI

```bash
orbit status              # Show all services and their status
orbit start nginx         # Start a service
orbit stop mariadb        # Stop a service
orbit install redis       # Install a service
orbit logs nginx          # View service logs
orbit db list             # List MariaDB databases
orbit hosts list          # Show hosts file entries
orbit config php 8.4      # Edit PHP config
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
