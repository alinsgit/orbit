# Blueprint GUI Wizard — Design

## Summary
Add a "From Blueprint" tab to the Add Site form, allowing one-click project creation from predefined blueprints (laravel-vite, nextjs-fullstack, django, etc.).

## Architecture

### Backend
- `services/blueprints.rs` — Blueprint struct + `get_blueprints()` (shared definition)
- `commands/blueprints.rs` — Two Tauri commands:
  - `list_blueprints` → returns all blueprints with metadata
  - `create_from_blueprint(blueprint, domain, path, php_version)` → creates site with correct template/dev_command, returns scaffold commands for terminal

### Frontend
- Add Site form gets two tabs: "Manual" | "Blueprint"
- Blueprint tab: card grid → select → domain/path input → Create
- After creation: open terminal with scaffold commands pre-filled

### Scaffold Strategy
Site creation is instant (Tauri command). Scaffold commands (npm, composer, pip) run in terminal — user sees output, can intervene on errors.

## Blueprint Data (8 blueprints)
laravel-vite, wordpress-woocommerce, nextjs-fullstack, astro-static, django, flask, sveltekit, remix
