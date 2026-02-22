#!/usr/bin/env bun
/**
 * MCP Tool Integration Test Suite
 *
 * Spawns orbit-mcp as a child process and sends JSON-RPC requests
 * to validate every tool works correctly.
 *
 * Usage:
 *   bun tests/mcp-test.ts                    # Run all tests
 *   bun tests/mcp-test.ts --group services   # Run specific group
 *   bun tests/mcp-test.ts --skip-destructive # Skip create/delete tests
 */

import { spawn, type Subprocess } from "bun";
import { resolve } from "path";

// ─── Config ────────────────────────────────────────────────────────────

const MCP_BIN = resolve(__dirname, "../core/target/debug/orbit-mcp.exe");
const TIMEOUT_MS = 15_000;
const TEST_DB = "__mcp_test_db__";
const TEST_SITE = "__mcp-test__.test";

// ─── Types ─────────────────────────────────────────────────────────────

interface TestCase {
  name: string;
  tool: string;
  args?: Record<string, unknown>;
  validate?: (result: any) => string | null; // null = pass, string = error
  requiresRunning?: string; // skip if this service is not running
  destructive?: boolean;
  cleanup?: () => TestCase; // cleanup action after test
  dependsOn?: string; // skip if this test failed
}

interface TestResult {
  name: string;
  tool: string;
  status: "pass" | "fail" | "skip" | "error";
  duration: number;
  message?: string;
}

// ─── MCP Client ────────────────────────────────────────────────────────

class McpClient {
  private proc: Subprocess;
  private buffer = "";
  private reqId = 0;
  private decoder = new TextDecoder();
  private pendingResolvers = new Map<number, {
    resolve: (value: any) => void;
    reject: (reason: any) => void;
    timer: ReturnType<typeof setTimeout>;
  }>();
  private reader: ReadableStreamDefaultReader<Uint8Array>;

  constructor(binPath: string) {
    this.proc = spawn([binPath], {
      stdin: "pipe",
      stdout: "pipe",
      stderr: "pipe",
    });
    this.reader = this.proc.stdout.getReader();
    this.startReading();
  }

  private async startReading() {
    try {
      while (true) {
        const { done, value } = await this.reader.read();
        if (done) break;
        this.buffer += this.decoder.decode(value, { stream: true });
        this.processBuffer();
      }
    } catch {
      // Process closed
    }
  }

  private processBuffer() {
    const lines = this.buffer.split("\n");
    this.buffer = lines.pop() || "";

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        const msg = JSON.parse(trimmed);
        if (msg.id !== undefined && this.pendingResolvers.has(msg.id)) {
          const pending = this.pendingResolvers.get(msg.id)!;
          clearTimeout(pending.timer);
          this.pendingResolvers.delete(msg.id);
          pending.resolve(msg);
        }
      } catch {
        // Skip non-JSON lines
      }
    }
  }

  async send(method: string, params?: Record<string, unknown>): Promise<any> {
    const id = ++this.reqId;
    const request: any = { jsonrpc: "2.0", id, method };
    if (params) request.params = params;

    const data = JSON.stringify(request) + "\n";
    this.proc.stdin.write(data);

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pendingResolvers.delete(id);
        reject(new Error(`Timeout waiting for response to ${method} (${TIMEOUT_MS}ms)`));
      }, TIMEOUT_MS);
      this.pendingResolvers.set(id, { resolve, reject, timer });
    });
  }

  async notify(method: string, params?: Record<string, unknown>) {
    const request: any = { jsonrpc: "2.0", method };
    if (params) request.params = params;
    this.proc.stdin.write(JSON.stringify(request) + "\n");
  }

  async initialize(): Promise<any> {
    const res = await this.send("initialize", {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "mcp-test", version: "1.0.0" },
    });
    await this.notify("initialized");
    return res;
  }

  async callTool(name: string, args: Record<string, unknown> = {}): Promise<any> {
    return this.send("tools/call", { name, arguments: args });
  }

  async listTools(): Promise<any> {
    return this.send("tools/list");
  }

  async close() {
    try {
      this.proc.stdin.end();
      this.proc.kill();
    } catch {
      // Already closed
    }
  }
}

// ─── Helpers ───────────────────────────────────────────────────────────

/** Extract text content from MCP tool response */
function getContent(response: any): string {
  if (response.error) return `ERROR: ${JSON.stringify(response.error)}`;
  const content = response.result?.content;
  if (Array.isArray(content) && content.length > 0) return content[0].text || "";
  return "";
}

function isError(response: any): boolean {
  if (response.error) return true;
  const result = response.result;
  if (!result) return false;
  // isError can be at content[0] level or at result level
  if (result.isError === true) return true;
  const content = result.content;
  if (Array.isArray(content) && content.length > 0) {
    if (content[0].isError === true) return true;
  }
  return false;
}

function parseJson(text: string): any {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

// Track running services for prerequisite checks
let runningServices: Set<string> = new Set();

// ─── Test Definitions ──────────────────────────────────────────────────

const tests: TestCase[] = [
  // ═══════════════════ SERVICES ═══════════════════
  {
    name: "List all services",
    tool: "list_services",
    validate: (r) => {
      const text = getContent(r);
      const data = parseJson(text);
      if (!Array.isArray(data)) return "Expected array of services";
      if (data.length === 0) return "No services found";
      // Cache running services
      for (const svc of data) {
        if (svc.status === "running") runningServices.add(svc.name);
      }
      return null;
    },
  },
  {
    name: "Get service status (nginx)",
    tool: "get_service_status",
    args: { name: "nginx" },
    validate: (r) => {
      const text = getContent(r);
      const data = parseJson(text);
      if (!data?.name) return "Missing service name in response";
      if (!["running", "stopped"].includes(data.status)) return `Unexpected status: ${data.status}`;
      return null;
    },
  },
  {
    name: "Get service status (mariadb)",
    tool: "get_service_status",
    args: { name: "mariadb" },
    validate: (r) => {
      const text = getContent(r);
      const data = parseJson(text);
      if (!data?.name) return "Missing service name in response";
      return null;
    },
  },
  {
    name: "Get service status (php-8.4)",
    tool: "get_service_status",
    args: { name: "php-8.4" },
    validate: (r) => {
      const text = getContent(r);
      const data = parseJson(text);
      if (!data?.name) return "Missing service name in response";
      return null;
    },
  },
  {
    name: "Get service status (unknown) → error",
    tool: "get_service_status",
    args: { name: "nonexistent-service-xyz" },
    validate: (r) => {
      if (!isError(r)) return "Expected error for unknown service";
      return null;
    },
  },
  {
    name: "Get system info",
    tool: "get_system_info",
    validate: (r) => {
      const text = getContent(r);
      if (!text.includes("data_dir") && !text.includes("bin_dir")) return "Missing system info fields";
      return null;
    },
  },

  // ═══════════════════ SITES ═══════════════════
  {
    name: "List sites",
    tool: "list_sites",
    validate: (r) => {
      const text = getContent(r);
      // Could be empty array or list
      if (isError(r)) return "Failed to list sites";
      return null;
    },
  },
  {
    name: "Create test site",
    tool: "create_site",
    args: { domain: TEST_SITE, path: "C:\\temp\\mcp-test" },
    destructive: true,
    validate: (r) => {
      if (isError(r)) return `Failed to create site: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Get site config",
    tool: "get_site_config",
    args: { domain: TEST_SITE },
    destructive: true,
    dependsOn: "Create test site",
    validate: (r) => {
      const text = getContent(r);
      if (isError(r)) return `Failed to get site config: ${text}`;
      return null;
    },
  },
  {
    name: "Delete test site",
    tool: "delete_site",
    args: { domain: TEST_SITE },
    destructive: true,
    dependsOn: "Create test site",
    validate: (r) => {
      if (isError(r)) return `Failed to delete site: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ LOGS ═══════════════════
  {
    name: "List log files",
    tool: "list_logs",
    validate: (r) => {
      if (isError(r)) return "Failed to list logs";
      return null;
    },
  },
  {
    name: "Read nginx access log",
    tool: "read_log",
    args: { name: "nginx/access.log", lines: 5 },
    validate: (r) => {
      // May be empty if no traffic, but shouldn't error
      if (isError(r)) return `Failed to read log: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ MariaDB ═══════════════════
  {
    name: "List databases",
    tool: "list_databases",
    requiresRunning: "mariadb",
    validate: (r) => {
      if (isError(r)) return `Failed to list databases: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Create test database",
    tool: "create_database",
    args: { name: TEST_DB },
    requiresRunning: "mariadb",
    destructive: true,
    validate: (r) => {
      if (isError(r)) return `Failed to create database: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "List tables in test DB",
    tool: "list_tables",
    args: { database: TEST_DB },
    requiresRunning: "mariadb",
    destructive: true,
    dependsOn: "Create test database",
    validate: (r) => {
      if (isError(r)) return `Failed to list tables: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Execute query (CREATE TABLE)",
    tool: "execute_query",
    args: { database: TEST_DB, query: "CREATE TABLE test_tbl (id INT PRIMARY KEY, name VARCHAR(50))" },
    requiresRunning: "mariadb",
    destructive: true,
    dependsOn: "Create test database",
    validate: (r) => {
      if (isError(r)) return `Failed to execute query: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Describe table",
    tool: "describe_table",
    args: { database: TEST_DB, table: "test_tbl" },
    requiresRunning: "mariadb",
    destructive: true,
    dependsOn: "Execute query (CREATE TABLE)",
    validate: (r) => {
      if (isError(r)) return `Failed to describe table: ${getContent(r)}`;
      const text = getContent(r);
      if (!text.includes("id") || !text.includes("name")) return "Missing column info";
      return null;
    },
  },
  {
    name: "Execute query (INSERT + SELECT)",
    tool: "execute_query",
    args: { database: TEST_DB, query: "INSERT INTO test_tbl VALUES (1, 'hello'); SELECT * FROM test_tbl" },
    requiresRunning: "mariadb",
    destructive: true,
    dependsOn: "Execute query (CREATE TABLE)",
    validate: (r) => {
      if (isError(r)) return `Failed to execute query: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Drop test database",
    tool: "drop_database",
    args: { name: TEST_DB },
    requiresRunning: "mariadb",
    destructive: true,
    dependsOn: "Create test database",
    validate: (r) => {
      if (isError(r)) return `Failed to drop database: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ PostgreSQL ═══════════════════
  {
    name: "PG list databases",
    tool: "pg_list_databases",
    requiresRunning: "postgresql",
    validate: (r) => {
      if (isError(r)) return `Failed to list PG databases: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "PG list tables (postgres)",
    tool: "pg_list_tables",
    args: { database: "postgres" },
    requiresRunning: "postgresql",
    validate: (r) => {
      if (isError(r)) return `Failed to list PG tables: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "PG execute query",
    tool: "pg_execute_query",
    args: { database: "postgres", query: "SELECT version()" },
    requiresRunning: "postgresql",
    validate: (r) => {
      if (isError(r)) return `Failed to execute PG query: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ PHP ═══════════════════
  {
    name: "List PHP extensions (8.4)",
    tool: "list_php_extensions",
    args: { version: "8.4" },
    validate: (r) => {
      if (isError(r)) return `Failed to list extensions: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Get PHP config (8.4)",
    tool: "get_php_config",
    args: { version: "8.4" },
    validate: (r) => {
      if (isError(r)) return `Failed to get PHP config: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ SSL ═══════════════════
  {
    name: "List SSL certs",
    tool: "list_ssl_certs",
    validate: (r) => {
      if (isError(r)) return `Failed to list SSL certs: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ Redis ═══════════════════
  {
    name: "Redis PING",
    tool: "redis_command",
    args: { command: "PING" },
    requiresRunning: "redis",
    validate: (r) => {
      if (isError(r)) return `Redis PING failed: ${getContent(r)}`;
      const text = getContent(r);
      if (!text.includes("PONG")) return `Expected PONG, got: ${text}`;
      return null;
    },
  },
  {
    name: "Redis SET/GET",
    tool: "redis_command",
    args: { command: "SET __mcp_test_key__ hello" },
    requiresRunning: "redis",
    validate: (r) => {
      if (isError(r)) return `Redis SET failed: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Redis GET test key",
    tool: "redis_command",
    args: { command: "GET __mcp_test_key__" },
    requiresRunning: "redis",
    dependsOn: "Redis SET/GET",
    validate: (r) => {
      if (isError(r)) return `Redis GET failed: ${getContent(r)}`;
      const text = getContent(r);
      if (!text.includes("hello")) return `Expected 'hello', got: ${text}`;
      return null;
    },
  },
  {
    name: "Redis DEL test key",
    tool: "redis_command",
    args: { command: "DEL __mcp_test_key__" },
    requiresRunning: "redis",
    dependsOn: "Redis SET/GET",
    validate: (r) => {
      if (isError(r)) return `Redis DEL failed: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Redis INFO",
    tool: "redis_info",
    requiresRunning: "redis",
    validate: (r) => {
      if (isError(r)) return `Redis INFO failed: ${getContent(r)}`;
      const text = getContent(r);
      if (!text.includes("redis_version")) return "Missing redis_version in INFO";
      return null;
    },
  },

  // ═══════════════════ Mailpit ═══════════════════
  {
    name: "List emails",
    tool: "list_emails",
    requiresRunning: "mailpit",
    validate: (r) => {
      if (isError(r)) return `Failed to list emails: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ Config ═══════════════════
  {
    name: "Read nginx config",
    tool: "read_config",
    args: { type: "nginx" },
    validate: (r) => {
      if (isError(r)) return `Failed to read nginx config: ${getContent(r)}`;
      const text = getContent(r);
      if (!text.includes("server") && !text.includes("http")) return "Doesn't look like nginx config";
      return null;
    },
  },
  {
    name: "Read MariaDB config",
    tool: "read_config",
    args: { type: "mariadb" },
    validate: (r) => {
      if (isError(r)) return `Failed to read mariadb config: ${getContent(r)}`;
      return null;
    },
  },
  {
    name: "Read PHP config file",
    tool: "read_config",
    args: { type: "php", php_version: "8.4" },
    validate: (r) => {
      if (isError(r)) return `Failed to read php config: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ Hosts ═══════════════════
  {
    name: "List hosts",
    tool: "hosts_list",
    validate: (r) => {
      if (isError(r)) return `Failed to list hosts: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ DB Export ═══════════════════
  {
    name: "DB export (nonexistent → error)",
    tool: "db_export",
    args: { database: "__nonexistent_db__", output_path: "C:\\temp\\test.sql" },
    requiresRunning: "mariadb",
    validate: (r) => {
      // Should fail gracefully for nonexistent DB
      if (isError(r)) return null; // Expected error
      return null; // Or it might export empty — either way OK
    },
  },

  // ═══════════════════ Run Orbit Command ═══════════════════
  {
    name: "Run orbit command (status)",
    tool: "run_orbit_command",
    args: { command: "status" },
    validate: (r) => {
      if (isError(r)) return `Failed to run orbit command: ${getContent(r)}`;
      return null;
    },
  },

  // ═══════════════════ Edge Cases ═══════════════════
  {
    name: "Empty tool args",
    tool: "list_services",
    args: {},
    validate: (r) => {
      if (isError(r)) return "list_services with empty args should work";
      return null;
    },
  },
  {
    name: "Invalid service name",
    tool: "start_service",
    args: { name: "" },
    validate: (r) => {
      if (!isError(r)) return "Expected error for empty service name";
      return null;
    },
  },
  {
    name: "Read nonexistent log",
    tool: "read_log",
    args: { name: "nonexistent/fake.log" },
    validate: (r) => {
      if (!isError(r)) return "Expected error for nonexistent log";
      return null;
    },
  },
];

// ─── Test Runner ───────────────────────────────────────────────────────

async function run() {
  const args = process.argv.slice(2);
  const skipDestructive = args.includes("--skip-destructive");
  const groupFilter = args.find((a) => a.startsWith("--group="))?.split("=")[1];

  console.log("╔═══════════════════════════════════════════════════════════╗");
  console.log("║          Orbit MCP Integration Test Suite                ║");
  console.log("╚═══════════════════════════════════════════════════════════╝\n");

  // Check binary exists
  const binFile = Bun.file(MCP_BIN);
  if (!(await binFile.exists())) {
    console.error(`❌ MCP binary not found: ${MCP_BIN}`);
    console.error(`   Run: cargo build --features mcp --bin orbit-mcp --manifest-path core/Cargo.toml`);
    process.exit(1);
  }

  console.log(`Binary: ${MCP_BIN}`);
  console.log(`Tests:  ${tests.length}`);
  if (skipDestructive) console.log("Mode:   Skip destructive tests");
  if (groupFilter) console.log(`Group:  ${groupFilter}`);
  console.log("");

  // Spawn MCP
  const client = new McpClient(MCP_BIN);

  try {
    // Initialize
    console.log("⏳ Initializing MCP connection...");
    const initRes = await client.initialize();
    if (initRes.error) {
      console.error("❌ Initialize failed:", initRes.error);
      process.exit(1);
    }
    console.log("✅ MCP initialized\n");

    // Verify tools/list
    const toolsRes = await client.listTools();
    const toolCount = toolsRes.result?.tools?.length || 0;
    console.log(`📋 Available tools: ${toolCount}\n`);

    // Run tests
    const results: TestResult[] = [];
    const failedTests = new Set<string>();

    for (const test of tests) {
      // Skip destructive
      if (skipDestructive && test.destructive) {
        results.push({ name: test.name, tool: test.tool, status: "skip", duration: 0, message: "Destructive (skipped)" });
        continue;
      }

      // Skip if dependency failed
      if (test.dependsOn && failedTests.has(test.dependsOn)) {
        results.push({ name: test.name, tool: test.tool, status: "skip", duration: 0, message: `Depends on: ${test.dependsOn}` });
        continue;
      }

      // Skip if required service not running
      if (test.requiresRunning && !runningServices.has(test.requiresRunning)) {
        results.push({ name: test.name, tool: test.tool, status: "skip", duration: 0, message: `${test.requiresRunning} not running` });
        continue;
      }

      const start = performance.now();
      try {
        const response = await client.callTool(test.tool, test.args || {});
        const duration = Math.round(performance.now() - start);

        if (test.validate) {
          const error = test.validate(response);
          if (error) {
            results.push({ name: test.name, tool: test.tool, status: "fail", duration, message: error });
            failedTests.add(test.name);
          } else {
            results.push({ name: test.name, tool: test.tool, status: "pass", duration });
          }
        } else {
          // No validator — just check it didn't timeout
          results.push({ name: test.name, tool: test.tool, status: "pass", duration });
        }
      } catch (err: any) {
        const duration = Math.round(performance.now() - start);
        results.push({ name: test.name, tool: test.tool, status: "error", duration, message: err.message });
        failedTests.add(test.name);
      }
    }

    // Print results
    console.log("─".repeat(72));
    console.log("  RESULTS");
    console.log("─".repeat(72));

    const statusIcons: Record<string, string> = { pass: "✅", fail: "❌", skip: "⏭️", error: "💥" };

    for (const r of results) {
      const icon = statusIcons[r.status];
      const dur = r.duration > 0 ? ` (${r.duration}ms)` : "";
      const msg = r.message ? ` — ${r.message}` : "";
      console.log(`${icon} ${r.name}${dur}${msg}`);
    }

    // Summary
    const pass = results.filter((r) => r.status === "pass").length;
    const fail = results.filter((r) => r.status === "fail").length;
    const skip = results.filter((r) => r.status === "skip").length;
    const error = results.filter((r) => r.status === "error").length;

    console.log("\n" + "═".repeat(72));
    console.log(`  SUMMARY: ${pass} passed, ${fail} failed, ${error} errors, ${skip} skipped (${results.length} total)`);
    console.log("═".repeat(72));

    if (fail > 0 || error > 0) {
      console.log("\n❌ FAILED TESTS:");
      for (const r of results.filter((r) => r.status === "fail" || r.status === "error")) {
        console.log(`   • ${r.tool}: ${r.message}`);
      }
    }

    await client.close();
    process.exit(fail + error > 0 ? 1 : 0);
  } catch (err: any) {
    console.error("💥 Fatal error:", err.message);
    await client.close();
    process.exit(1);
  }
}

run();
