# Orbit Ecosystem Expansion Roadmap

This document serves as the master blueprint for transforming Orbit from a standard local server manager into a full-fledged local development environment.

## Phase 1: Service Expansion (orbit-libraries)

We are adding new fetch scripts to `orbit-libraries` to dynamically download the latest versions of these tools:

- **Databases**: PostgreSQL, MongoDB
- **Runtimes & Languages**: Go (Golang), Deno, Rust (rustup)
- **Tools**: Ngrok (for local tunneling)

## Phase 2: Orbit Terminal (Isolated Environment)

Building an integrated terminal inside the Orbit UI (likely using `xterm.js` or a Tauri native terminal).

- **Goal**: When a user opens the Orbit Terminal, the `PATH` environment variable is temporarily overridden so that commands like `php`, `composer`, `node`, `mysql`, `psql`, etc., resolve strictly to the versions currently active in the Orbit environment, preventing conflicts with global system installations.

## Phase 3: Project Wizards & Reverse Proxy

Implementing 1-Click project scaffolding and routing for modern frameworks.

- **Backend Wizards**: Laravel (`composer create-project`), WordPress.
- **Frontend/Fullstack Wizards**: Astro, Next.js, Nuxt, SvelteKit.
- **Infrastructure**: Implement a `TEMPLATE_REVERSE_PROXY` in Nginx to route traffic from `local` domains to the respective dev server ports (e.g., 5173, 3000) spawned by the Javascript frameworks.
