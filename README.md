# Orbit

Modern local development environment. Manage Nginx, PHP, MariaDB, Redis and more from a single interface.

## Features

- **Service Management** - Install, start, stop and configure services with one click
- **Site Management** - Create and manage local sites with custom domains and SSL
- **Database Tools** - Built-in Adminer and phpMyAdmin integration
- **Log Viewer** - Real-time log monitoring with filtering by service and level
- **PATH Management** - Add/remove services from system PATH per service
- **Multi-version PHP** - Run multiple PHP versions side by side

### Supported Services

Nginx, Apache, PHP, MariaDB, Redis, Node.js, Python, Bun, Composer, Mailpit

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
core/    Rust (Tauri) backend
```

## License

[MIT](LICENSE)
