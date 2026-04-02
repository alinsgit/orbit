# Contributing to Orbit

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

### Prerequisites
- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) (latest)
- Windows 10/11

### Getting Started

```bash
git clone https://github.com/alinsgit/orbit.git
cd orbit
bun install
bun tauri dev
```

### Project Structure

```
app/          # React + TypeScript frontend
core/         # Rust backend (Tauri 2)
  src/
    commands/ # IPC handlers
    services/ # Business logic
docs/         # Landing page (GitHub Pages)
scripts/      # Build utilities
```

## How to Contribute

### Bug Reports
- Use the **Bug Report** issue template
- Include OS version, Orbit version, and steps to reproduce
- Attach relevant logs from the Logs tab

### Feature Requests
- Use the **Feature Request** issue template
- Describe the use case, not just the solution

### Pull Requests
1. Fork the repo and create your branch from `main`
2. Make your changes
3. Ensure `bun run build` passes (frontend)
4. Ensure `cargo clippy` passes (backend)
5. Write a clear PR description

### Code Standards
- **Frontend:** TypeScript, React 19, TailwindCSS v4, 2-space indent
- **Backend:** Rust, 2-space indent
- **Naming:** kebab-case (files), camelCase (variables), PascalCase (components)

## Building

```bash
bun run build                    # Frontend only
cd core && cargo build           # Backend only
bun tauri build                  # Full production build
cd core && cargo build --features mcp --bin orbit-mcp  # MCP server
```

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
