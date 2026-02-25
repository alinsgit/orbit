# Orbit — Test Suite Development Prompt

## Project Overview

Orbit is a Windows local development environment manager built with **Tauri 2** (Rust backend + React 19 frontend). It manages 16 services: Nginx, Apache, PHP (multi-version), MariaDB, PostgreSQL, MongoDB, Redis, Node.js, Python, Bun, Deno, Go, Rust, Mailpit, Composer, Ngrok.

**Architecture**: 3 binaries share the `core/` crate:
- **Tauri GUI** (`main.rs` + `lib.rs`) — desktop app
- **CLI** (`cli.rs`) — standalone binary, no Tauri/tokio dependency
- **MCP** (`mcp.rs`) — MCP server over stdio, no Tauri/tokio dependency

## Goal

Write comprehensive Rust unit tests and integration tests for the `core/` crate. There are currently **zero** Rust tests (only a TypeScript MCP integration test exists at `tests/mcp-test.ts`).

## Build Commands

```bash
cargo test                                          # Run all tests
cargo build                                         # Tauri app
cargo build --features mcp --bin orbit-mcp          # MCP binary
```

## Test Infrastructure Setup

Add these dev-dependencies to `core/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

## File Structure

```
core/src/
  main.rs                    # Tauri entry
  lib.rs                     # Plugin init, command registration
  cli.rs                     # CLI binary (clap + colored)
  mcp.rs                     # MCP server (68 tools, JSON-RPC 2.0)
  commands/                  # Tauri invoke handlers (~25 modules)
  services/                  # Business logic (~30 modules)
    validation.rs            # Input validation — ALL PURE FUNCTIONS
    templates.rs             # Nginx/Apache template engine — MOSTLY PURE
    site_store.rs            # JSON site storage — PURE in-memory operations
    blueprints.rs            # Blueprint definitions — PURE
    process.rs               # Service process management — port calc is PURE
    config.rs                # Config file generation — needs tempdir
    hosts.rs                 # Hosts file management — needs tempdir
    sites.rs                 # Site CRUD — needs AppHandle (integration)
    nginx.rs                 # Nginx config generation
    apache.rs                # Apache config generation
    php_registry.rs          # PHP version registry
    ...
```

## PRIORITY 1: Pure Function Tests (No I/O, No Mocking)

### 1a. `services/validation.rs`

This file has 8+ pure validation functions. There are 3 existing tests at the bottom of the file — expand them significantly.

**Functions to test:**

```rust
pub fn validate_domain(domain: &str) -> Result<(), ValidationError>
// RFC 1123 compliant. Test cases needed:
// - Valid: "example.test", "my-site.local", "sub.domain.test", "a.b"
// - Invalid: "" (empty), "a" * 254 (too long), ".leading-dot", "trailing-dot.",
//   "double..dot", "-leading-hyphen.test", "UPPER.test" (should pass, case-insensitive)
// - Dangerous: "../traversal", "localhost", "127.0.0.1", "; rm -rf", "domain\ninjection"
// - Edge: single char labels, max length labels (63 chars), unicode

pub fn validate_port(port: u16) -> Result<(), ValidationError>
// Valid: 80, 443, 1024, 3000, 8080, 65535
// Invalid: 0, 1, 79

pub fn validate_php_version(version: &str) -> Result<(), ValidationError>
// Valid: "8.4", "8.4.1", "7.4", "8.0"
// Invalid: "", "abc", "8", ".4", "8.4;"

pub fn validate_ini_key(key: &str) -> Result<(), ValidationError>
// Valid: "max_execution_time", "upload_max_filesize", "memory_limit"
// Invalid: "", "key with spaces", "key;injection", "disable_functions" (blocked)

pub fn validate_ini_value(value: &str) -> Result<(), ValidationError>
// Valid: "128M", "On", "0", "/tmp"
// Invalid: "value\nnewline", "value\0null", "${ENV_VAR}", "$(command)"

pub fn sanitize_for_nginx(value: &str) -> String
// Removes: ; { } " ' $ ` \ | > <
// Test: "normal" → "normal", "test;rm" → "testrm", "a{b}c" → "abc"

pub fn sanitize_for_powershell(value: &str) -> String
// Doubles single quotes, removes dangerous chars
// Test: "it's" → "it''s", "test`cmd" → "testcmd"

pub fn validate_site_path(path: &str, _allowed_base: Option<&Path>) -> Result<PathBuf, ValidationError>
// Pure part: null bytes, traversal detection
// Test traversal: "../../../etc/passwd", "..\\windows\\system32"
// Test null: "path\0injection"
// Note: Also calls .exists() — for pure tests, only test the rejection cases
```

**Target: 50+ test cases for validation.rs**

### 1b. `services/templates.rs`

```rust
// Template rendering — PURE string replacement
impl TemplateEngine {
    pub fn render(template: &str, vars: &HashMap<&str, String>) -> String
}
// Test: all 6 variables: {{DOMAIN}}, {{ROOT_PATH}}, {{PORT}}, {{PHP_PORT}}, {{SSL_CERT}}, {{SSL_KEY}}
// Test: missing variables (should remain as-is)
// Test: multiple occurrences of same variable
// Test: empty template, empty vars

// Template type detection
impl SiteTemplate {
    pub fn get_nginx_template(&self) -> &'static str    // 10 match arms
    pub fn get_apache_template(&self) -> &'static str   // 6 match arms
}
// Test: each template type returns non-empty string
// Test: templates contain expected placeholders

// Template type from string
// Test: "http" → Http, "laravel" → Laravel, "unknown" → Http (default)
```

**Target: 25+ test cases for templates.rs**

### 1c. `services/site_store.rs`

In-memory operations are pure:
```rust
impl SiteStore {
    pub fn add_site(&mut self, site: SiteMetadata)      // deduplicates by domain
    pub fn remove_site(&mut self, domain: &str) -> Option<SiteMetadata>
    pub fn get_site(&self, domain: &str) -> Option<&SiteMetadata>
    pub fn get_site_mut(&mut self, domain: &str) -> Option<&mut SiteMetadata>
    pub fn update_site(&mut self, domain: &str, updates: SiteMetadata) -> bool
}
```

Create a `SiteStore` with `version: "1".to_string(), sites: vec![]`, then test CRUD.

**Target: 15+ test cases**

### 1d. `services/blueprints.rs`

```rust
pub fn get_blueprints() -> Vec<Blueprint>
```
- Test: returns 8 blueprints
- Test: each blueprint has non-empty name, description, services, template
- Test: each blueprint's template is a valid template name
- Test: PHP blueprints have php_extensions, non-PHP don't
- Test: dev_command is set for all except wordpress-woocommerce

**Target: 10+ test cases**

### 1e. `services/process.rs` — Port Calculation

```rust
fn get_service_port(service_name: &str) -> Option<u16>
```

**CRITICAL PORT MAPPING** (past bugs here!):
- "nginx" → 80
- "apache" → 80
- "mariadb" → 3306
- "postgresql" / "postgres" → 5432
- "mongodb" → 27017
- "redis" → 6379
- "mailpit" → 8025
- "php-8.4" → 9004 (9000 + minor version)
- "php-8.0" → 9000
- "php-7.4" → 9004
- "php-8.5" → 9005
- "unknown" → None

**IMPORTANT**: PHP port = `9000 + minor_version`. NOT `9000 + all_digits`. This was a critical bug in v1.0.0.

Also test:
```rust
fn get_process_names(service_name: &str) -> Vec<&'static str>
```
- "nginx" → ["nginx.exe"]
- "php-8.4" → ["php-cgi.exe"]
- "mariadb" → ["mysqld.exe"]

**Target: 20+ test cases**

## PRIORITY 2: Integration Tests with tempdir

### 2a. `services/config.rs`

```rust
impl ConfigManager {
    pub fn ensure_nginx_config(nginx_root: &PathBuf) -> Result<(), String>
    pub fn ensure_mariadb_config(mariadb_root: &PathBuf) -> Result<(), String>
    pub fn ensure_php_config(php_root: &PathBuf) -> Result<(), String>
    pub fn ensure_apache_config(apache_root: &PathBuf) -> Result<(), String>
}
```

Use `tempfile::tempdir()` for each test:
- Test: creates expected files (nginx.conf, mime.types, etc.)
- Test: idempotent — calling twice doesn't error
- Test: created files contain expected content snippets

**Target: 12+ test cases**

### 2b. `services/hosts.rs`

For `add_domain` and `remove_domain`, these modify `C:\Windows\System32\drivers\etc\hosts`.
Skip these in CI (they need admin privileges). But you CAN test the domain validation part:
- `add_domain("")` should fail with validation error
- `add_domain("../evil")` should fail

**Target: 5+ test cases (validation only)**

## PRIORITY 3: Serialization / Schema Tests

### 3a. Site/Blueprint/SiteMetadata serde tests

```rust
// Test: Site serializes to JSON and deserializes back correctly
// Test: SiteWithStatus serializes with all fields
// Test: Blueprint serializes with all fields
// Test: SiteStore round-trip (serialize → deserialize)
// Test: Optional fields (None → absent in JSON, present → round-trips)
```

**Target: 10+ test cases**

## Test Organization

Place unit tests in each module using `#[cfg(test)]` blocks:

```rust
// At the bottom of services/validation.rs:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain_valid() {
        assert!(validate_domain("example.test").is_ok());
    }
    // ...
}
```

For integration tests that need tempdir, create `core/tests/` directory:
```
core/tests/
  config_test.rs      # ConfigManager integration tests
  store_test.rs       # SiteStore persistence tests
```

## Important Constraints

1. **No Tauri runtime in tests** — Don't try to create `AppHandle`. Test pure functions and mock-free logic only.
2. **Windows paths** — Use `std::path::PathBuf` for cross-platform. Some tests may need `#[cfg(target_os = "windows")]`.
3. **No network calls** — Don't test functions that call external APIs or spawn processes.
4. **Existing tests** — `validation.rs` has 3 existing tests at the bottom. Don't duplicate them, extend them.
5. **Private functions** — Some functions are private. Either make them `pub(crate)` for testing, or test them through their public API.
6. **Feature flags** — MCP tests need `#[cfg(feature = "mcp")]`. CLI tests need `#[cfg(feature = "cli")]`. Most service tests don't need any feature flag.

## Expected Output

- **~130+ test cases** total across all priority levels
- All tests pass with `cargo test`
- Tests are well-organized with descriptive names
- Edge cases and error conditions are covered
- No flaky tests (no timing, no network, no external state)

## Verification

After writing all tests, run:
```bash
cd core
cargo test 2>&1
```

All tests must pass. Report the final count.
