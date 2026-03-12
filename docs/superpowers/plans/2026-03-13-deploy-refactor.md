# Deploy System Refactor — Global Connections + Site Targets

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor deploy system so server connections are global (defined once in Settings) and sites reference them with a remote_path.

**Architecture:** Split `DeployConnection` into two concepts: global `ServerConnection` (host/auth) and per-site `DeployTarget` (connection ref + remote_path). Protocol simplified from SSH/SFTP/FTP to SSH/FTP (SSH covers both command execution and SFTP file transfer).

**Tech Stack:** Rust (Tauri commands, keyring, ssh2, suppaftp), React + TypeScript frontend

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Rewrite | `core/src/services/deploy_store.rs` | Global connection CRUD + site target CRUD + keyring |
| Modify | `core/src/services/deploy.rs` | Update sync/test/execute to resolve connection from target |
| Rewrite | `core/src/commands/deploy.rs` | New IPC handlers matching new API surface |
| Modify | `core/src/lib.rs:297-304` | Update command registration |
| Modify | `core/src/mcp.rs:1903-1962,2246-2271,5211-5609` | Update MCP tool definitions and handlers |
| Rewrite | `app/lib/api/deploy.ts` | New interfaces + API functions |
| Modify | `app/components/SettingsManager.tsx` | Add "Connections" section for global connection CRUD |
| Rewrite | `app/components/DeployPanel.tsx` | Connection selector + remote_path instead of full form |
| Modify | `app/components/SitesManager.tsx:2145-2173` | Minor — DeployPanel props may change |

---

## Chunk 1: Backend Data Model & Commands

### Task 1: Rewrite deploy_store.rs — new data model + storage

**Files:**
- Rewrite: `core/src/services/deploy_store.rs`

New structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnection {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    SSH,  // covers both SSH commands and SFTP file transfer
    FTP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    KeyFile(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployTarget {
    pub connection: String,   // references ServerConnection.name
    pub remote_path: String,
}
```

Storage functions:

```rust
impl DeployStore {
    // ── Paths ──
    fn connections_path(app: &AppHandle) -> PathBuf  // config/deploy-connections.json
    fn targets_path(app: &AppHandle) -> PathBuf      // config/deploy-targets.json
    fn keyring_key(conn_name: &str) -> String         // "orbit:deploy:{conn_name}"

    // ── Global Connections ──
    pub fn list_connections(app: &AppHandle) -> Vec<ServerConnection>
    pub fn get_connection(app: &AppHandle, name: &str) -> Option<ServerConnection>
    pub fn add_connection(app: &AppHandle, conn: ServerConnection, password: Option<String>) -> Result<(), String>
    pub fn remove_connection(app: &AppHandle, name: &str) -> Result<(), String>
    pub fn get_password(conn_name: &str) -> Result<String, String>

    // ── Site Targets ──
    pub fn list_targets(app: &AppHandle, domain: &str) -> Vec<DeployTarget>
    pub fn assign_target(app: &AppHandle, domain: &str, target: DeployTarget) -> Result<(), String>
    pub fn unassign_target(app: &AppHandle, domain: &str, conn_name: &str) -> Result<(), String>

    // ── Migration ──
    pub fn migrate_if_needed(app: &AppHandle)  // converts old format to new on first load
}
```

- [ ] **Step 1:** Write the new `deploy_store.rs` with all structs and functions above. Migration function reads old `deploy-connections.json` (HashMap<domain, Vec<old_format>>), extracts unique connections into global list, creates targets per domain, writes both new files, renames old file to `.bak`.

- [ ] **Step 2:** `cargo build` — verify compiles (will have unused warnings, that's fine)

- [ ] **Step 3:** Commit: `refactor: new deploy data model with global connections + site targets`

### Task 2: Rewrite commands/deploy.rs — new IPC handlers

**Files:**
- Rewrite: `core/src/commands/deploy.rs`
- Modify: `core/src/lib.rs:297-304`

New commands:

```rust
// Global connections
#[tauri::command]
pub fn deploy_list_connections(app: AppHandle) -> Result<Vec<ServerConnection>, String>

#[tauri::command]
pub fn deploy_add_connection(app: AppHandle, connection: ServerConnection, password: Option<String>) -> Result<String, String>

#[tauri::command]
pub fn deploy_remove_connection(app: AppHandle, name: String) -> Result<String, String>

#[tauri::command]
pub fn deploy_test_connection(app: AppHandle, name: String) -> Result<String, String>

// Site targets
#[tauri::command]
pub fn deploy_list_targets(app: AppHandle, domain: String) -> Result<Vec<DeployTarget>, String>

#[tauri::command]
pub fn deploy_assign_target(app: AppHandle, domain: String, connection: String, remote_path: String) -> Result<String, String>

#[tauri::command]
pub fn deploy_unassign_target(app: AppHandle, domain: String, connection: String) -> Result<String, String>

// Operations
#[tauri::command]
pub fn deploy_sync(app: AppHandle, domain: String, connection: String, site_path: String) -> Result<DeployManifest, String>

#[tauri::command]
pub fn deploy_ssh_execute(app: AppHandle, connection: String, command: String) -> Result<String, String>

#[tauri::command]
pub fn deploy_get_status(app: AppHandle, domain: String, connection: String) -> Result<Option<DeployManifest>, String>
```

- [ ] **Step 1:** Write the new `commands/deploy.rs`. `deploy_sync` resolves connection from store, gets target's remote_path, delegates to DeployService. `deploy_ssh_execute` no longer needs domain (connection is global). `deploy_test_connection` no longer needs domain.

- [ ] **Step 2:** Update `lib.rs` command registration — replace old 7 commands with new 10 commands. Add `deploy_list_targets`, `deploy_assign_target`, `deploy_unassign_target`.

- [ ] **Step 3:** `cargo build` — verify compiles

- [ ] **Step 4:** Commit: `refactor: deploy commands with global connections + site targets API`

### Task 3: Update deploy.rs — sync engine adapts to new model

**Files:**
- Modify: `core/src/services/deploy.rs`

Changes:
- `test_connection()`: Takes `&ServerConnection` instead of looking up by domain+name
- `create_ssh_session()`: Takes `&ServerConnection` directly
- `ssh_execute()`: Takes `&ServerConnection` + command
- `sync_sftp()` / `sync_ftp()`: Takes `&ServerConnection` + `remote_path: &str` + `site_path` + `domain` (for manifest/lock)
- Remove `Protocol::SFTP` references — treat SSH connections as SFTP-capable
- All internal functions receive resolved data, no more store lookups

- [ ] **Step 1:** Update function signatures in `deploy.rs`. Replace `DeployConnection` parameter types with `ServerConnection`. Remove SFTP enum matching (SSH handles both). The `sync_sftp` and `sync_ftp` functions receive `remote_path` as a separate parameter instead of reading from connection.

- [ ] **Step 2:** `cargo build` — verify compiles

- [ ] **Step 3:** Commit: `refactor: deploy engine uses resolved ServerConnection + remote_path`

---

## Chunk 2: Frontend API & Settings UI

### Task 4: Rewrite api/deploy.ts — new interfaces

**Files:**
- Rewrite: `app/lib/api/deploy.ts`

```typescript
export interface ServerConnection {
  name: string
  host: string
  port: number
  username: string
  auth: 'Password' | { KeyFile: string }
  protocol: 'SSH' | 'FTP'
}

export interface DeployTarget {
  connection: string
  remote_path: string
}

export interface DeployManifest {
  timestamp: string
  domain: string
  connection: string
  files: { path: string; hash: string; size: number }[]
  status: 'InProgress' | 'Completed' | { Failed: string }
}

// Global connections
export function deployListConnections(): Promise<ServerConnection[]>
export function deployAddConnection(connection: ServerConnection, password?: string): Promise<string>
export function deployRemoveConnection(name: string): Promise<string>
export function deployTestConnection(name: string): Promise<string>

// Site targets
export function deployListTargets(domain: string): Promise<DeployTarget[]>
export function deployAssignTarget(domain: string, connection: string, remotePath: string): Promise<string>
export function deployUnassignTarget(domain: string, connection: string): Promise<string>

// Operations
export function deploySync(domain: string, connection: string, sitePath: string): Promise<DeployManifest>
export function deploySshExecute(connection: string, command: string): Promise<string>
export function deployGetStatus(domain: string, connection: string): Promise<DeployManifest | null>
```

- [ ] **Step 1:** Write the new `api/deploy.ts` with all interfaces and invoke wrappers.

- [ ] **Step 2:** `bun run build` — will fail because DeployPanel still uses old interfaces. That's expected, continue to next task.

- [ ] **Step 3:** Commit: `refactor: deploy API with global connections + site targets`

### Task 5: Add Connections section to SettingsManager

**Files:**
- Modify: `app/components/SettingsManager.tsx`

Add a "Server Connections" section in the settings grid. Compact list of connections with inline add form. Pattern:

- List existing connections (name, host, username, protocol badge)
- Icon-only action buttons (test, delete) — same compact style as tool cards
- Collapsible "Add Connection" form: name, host, port, username, SSH/FTP toggle, password/keyfile, save/cancel
- Uses `deployListConnections`, `deployAddConnection`, `deployRemoveConnection`, `deployTestConnection`

- [ ] **Step 1:** Add imports and state for connections in SettingsManager.

- [ ] **Step 2:** Add the Connections section component/JSX after "Workspace & General" section. Compact card style consistent with existing settings sections. Form fields: name, host:port (inline), username, protocol (SSH/FTP toggle), auth type (password/keyfile toggle), credential input.

- [ ] **Step 3:** `bun run build` — may still fail from DeployPanel, that's ok

- [ ] **Step 4:** Commit: `feat: server connections management in Settings`

### Task 6: Rewrite DeployPanel — connection selector

**Files:**
- Rewrite: `app/components/DeployPanel.tsx`

New DeployPanel is much simpler:
- Load global connections via `deployListConnections()`
- Load site targets via `deployListTargets(domain)`
- Show assigned targets as compact rows (connection name + remote_path + action icons)
- "Add Target" form: dropdown to select global connection + remote_path input
- Deploy/test/remove buttons per target (icon-only)
- Keep progress bar and last deploy status display

Props stay the same: `{ domain: string, sitePath: string }`

- [ ] **Step 1:** Rewrite DeployPanel with connection selector dropdown + remote_path input. Remove all credential form fields.

- [ ] **Step 2:** `bun run build` — should pass now

- [ ] **Step 3:** Commit: `refactor: DeployPanel uses global connections with target selector`

---

## Chunk 3: MCP & Migration

### Task 7: Update MCP deploy tools

**Files:**
- Modify: `core/src/mcp.rs` (lines 1903-1962 tool defs, 2246-2271 dispatch, 5211-5609 handlers)

Update tool definitions:
- `deploy_list_connections` — no longer takes `domain` param
- `deploy_test_connection` — takes `connection_name` only (no domain)
- `deploy_ssh_execute` — takes `connection_name` + `command` (no domain)
- `deploy_sync` — takes `domain` + `connection_name` (resolves target's remote_path)
- `deploy_status` — unchanged
- Add: `deploy_list_targets` — takes `domain`, returns targets
- Add: `deploy_assign_target` — takes `domain`, `connection_name`, `remote_path`
- Add: `deploy_unassign_target` — takes `domain`, `connection_name`

Update handler functions to use new storage format (read connections.json as Vec, targets.json as HashMap).

- [ ] **Step 1:** Update MCP tool definitions (inputSchema) for all deploy tools. Add 3 new tool definitions.

- [ ] **Step 2:** Update dispatch section to route new tools.

- [ ] **Step 3:** Update all `tool_deploy_*` handler functions to use new data model. Connection lookup is now global (not per-domain). Sync function resolves remote_path from targets file.

- [ ] **Step 4:** `cargo build --features mcp --bin orbit-mcp` — verify compiles

- [ ] **Step 5:** Commit: `refactor: MCP deploy tools use global connections + targets`

### Task 8: Migration logic + full build verification

**Files:**
- Verify: `core/src/services/deploy_store.rs` (migrate_if_needed)

- [ ] **Step 1:** Ensure `migrate_if_needed()` is called from `deploy_list_connections` command (first access triggers migration). Old `deploy-connections.json` with `HashMap<domain, Vec<OldConnection>>` format detected by checking if JSON root is an object with domain keys containing arrays with `remote_path` field in connection objects.

- [ ] **Step 2:** Full build: `cargo build` + `bun run build` — both must pass clean

- [ ] **Step 3:** `cargo clippy` — fix any warnings

- [ ] **Step 4:** Commit: `chore: deploy migration + build verification`

---

## Chunk 4: CLI Update

### Task 9: Update CLI deploy commands (if any)

**Files:**
- Check: `core/src/cli.rs`

- [ ] **Step 1:** Check if CLI has deploy commands. If yes, update to match new data model. If no, skip.

- [ ] **Step 2:** Final full build: `cargo build` + `cargo build --features mcp --bin orbit-mcp` + `bun run build`

- [ ] **Step 3:** Commit if changes made: `refactor: CLI deploy commands use global connections`
