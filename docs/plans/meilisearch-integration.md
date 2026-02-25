# Meilisearch Integration Plan

> Meilisearch — hafif, hızlı full-text search engine. Daemon olarak eklenir (port 7700).
> Mailpit pattern'i temel alınır (en basit daemon).

## Dosya Listesi

### Yeni Dosyalar (2)
- `core/src/services/meilisearch.rs` — MeilisearchManager struct + install/start/stop/status
- `core/src/commands/meilisearch.rs` — Tauri invoke handler'ları

### Değişiklik Yapılacak Dosyalar (10)
| Dosya | Değişiklik |
|---|---|
| `core/src/services/mod.rs` | `pub mod meilisearch;` |
| `core/src/commands/mod.rs` | `pub mod meilisearch;` |
| `core/src/services/process.rs` | ServiceType enum + port (7700) + process name + stop_all |
| `core/src/lib.rs` | invoke_handler'a 5-6 yeni komut |
| `core/src/mcp.rs` | scan_services, port, process, start match |
| `core/src/cli.rs` | scan_services, port, process, start match, alias "meili" |
| `core/dist/libraries.json` | Meilisearch download URL entry |
| `app/components/ServiceManager.tsx` | SERVICE_CATALOG + STARTABLE_TYPES |
| `app/lib/api.ts` | Typed invoke wrappers |
| `app/lib/serviceIcons.ts` | Icon mapping |

## Görevler

### Task 1: Registry + Backend Service Module
1. `core/dist/libraries.json` — Meilisearch entry ekle
   - Windows: `meilisearch-windows-amd64.exe` (GitHub releases)
   - Single exe, zip/extract yok
2. `core/src/services/meilisearch.rs` — MeilisearchManager
   - `get_dir()`, `get_exe_path()`, `is_installed()`, `is_running()`
   - `install()` — registry'den URL al, exe indir, rename
   - `start()` — `hidden_command()` ile spawn, args: `--http-addr 127.0.0.1:7700 --db-path data.ms`
   - `stop()` — `taskkill /F /IM meilisearch.exe`
   - `uninstall()` — dizin sil
   - `get_status()` → MeilisearchStatus struct
3. `core/src/services/mod.rs` — mod ekle
4. `core/src/services/process.rs` — ServiceType::Meilisearch, port 7700, process name

### Task 2: Tauri Commands
1. `core/src/commands/meilisearch.rs` — 5-6 command
   - `get_meilisearch_status`, `install_meilisearch`, `uninstall_meilisearch`
   - `start_meilisearch`, `stop_meilisearch`, `get_meilisearch_exe_path`
2. `core/src/commands/mod.rs` — mod ekle
3. `core/src/lib.rs` — invoke_handler'a kaydet

### Task 3: MCP + CLI
1. `core/src/mcp.rs`:
   - `scan_services()` — meilisearch.exe detection
   - `get_service_port()` — 7700
   - `get_process_image_names()` — "meilisearch.exe"
   - `start_service_process()` — match arm
2. `core/src/cli.rs`:
   - Aynı 4 nokta
   - `resolve_service_name()` — "meili" alias

### Task 4: Frontend
1. `app/lib/api.ts` — invoke wrapper'ları
2. `app/lib/serviceIcons.ts` — icon (🔍)
3. `app/components/ServiceManager.tsx`:
   - SERVICE_CATALOG entry (group: 'server')
   - STARTABLE_TYPES'a ekle

### Task 5: Build Verification
- `cargo build --release`
- `cargo build --features mcp --bin orbit-mcp`
- `cargo build --bin orbit-cli`
- Frontend: `bun run build`

## Notlar
- Meilisearch varsayılan port: **7700** (HTTP API + dashboard)
- Web dashboard: `http://localhost:7700` — ayrı UI panel'e gerek yok, tarayıcıdan erişilir
- Data path: `bin/meilisearch/data.ms/`
- Master key opsiyonel (local dev için gerek yok)
- Meilisearch tek binary, dependency yok — en temiz daemon
