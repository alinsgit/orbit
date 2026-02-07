import Database from '@tauri-apps/plugin-sql';

// Connection state
let db: Database | null = null;

export interface DatabaseInfo {
  name: string;
  size?: string;
  tables?: number;
}

export interface UserInfo {
  user: string;
  host: string;
}

export interface ServerInfo {
  version: string;
  uptime: number;
  connections: number;
}

export interface DbConnectionConfig {
  host: string;
  port: number;
  user: string;
  password: string;
}

// Connect to MariaDB
export async function dbConnect(config: DbConnectionConfig): Promise<void> {
  // Security: Only allow localhost connections
  if (config.host !== '127.0.0.1' && config.host !== 'localhost') {
    throw new Error('Only localhost connections are allowed');
  }

  // Connect to 'mysql' system database by default (always exists)
  const connectionString = `mysql://${encodeURIComponent(config.user)}:${encodeURIComponent(config.password)}@${config.host}:${config.port}/mysql`;

  try {
    db = await Database.load(connectionString);
  } catch (error) {
    throw new Error(`Connection failed: ${error}`);
  }
}

// Disconnect
export async function dbDisconnect(): Promise<void> {
  if (db) {
    await db.close();
    db = null;
  }
}

// Check connection
export function isConnected(): boolean {
  return db !== null;
}

// Get server info
export async function getServerInfo(): Promise<ServerInfo> {
  if (!db) throw new Error('Not connected');

  const versionResult = await db.select<Record<string, string>[]>("SHOW VARIABLES LIKE 'version'");
  const uptimeResult = await db.select<Record<string, string>[]>("SHOW STATUS LIKE 'Uptime'");
  const connectionsResult = await db.select<Record<string, string>[]>("SHOW STATUS LIKE 'Threads_connected'");

  // SHOW VARIABLES returns: Variable_name, Value
  const getVal = (result: Record<string, string>[]) => {
    if (!result[0]) return '';
    return result[0].Value || result[0].value || Object.values(result[0])[1] || '';
  };

  return {
    version: getVal(versionResult) || 'Unknown',
    uptime: parseInt(getVal(uptimeResult) || '0'),
    connections: parseInt(getVal(connectionsResult) || '0'),
  };
}

// List databases
export async function listDatabases(): Promise<DatabaseInfo[]> {
  if (!db) throw new Error('Not connected');

  const result = await db.select<Record<string, string>[]>('SHOW DATABASES');

  return result.map(row => ({
    name: row.Database || Object.values(row)[0] || 'unknown',
  }));
}

// Create database
export async function createDatabase(
  name: string,
  charset: string = 'utf8mb4',
  collation: string = 'utf8mb4_unicode_ci'
): Promise<void> {
  if (!db) throw new Error('Not connected');

  // Validate database name (prevent SQL injection)
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(name)) {
    throw new Error('Invalid database name');
  }

  await db.execute(`CREATE DATABASE \`${name}\` CHARACTER SET ${charset} COLLATE ${collation}`);
}

// Drop database
export async function dropDatabase(name: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  // Prevent dropping system databases
  const systemDbs = ['mysql', 'information_schema', 'performance_schema', 'sys'];
  if (systemDbs.includes(name.toLowerCase())) {
    throw new Error('Cannot drop system database');
  }

  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(name)) {
    throw new Error('Invalid database name');
  }

  await db.execute(`DROP DATABASE \`${name}\``);
}

// List users
export async function listUsers(): Promise<UserInfo[]> {
  if (!db) throw new Error('Not connected');

  try {
    // CAST to VARCHAR to avoid BINARY type issue
    const result = await db.select<Record<string, string>[]>(
      'SELECT CAST(User AS CHAR) as user, CAST(Host AS CHAR) as host FROM mysql.user'
    );

    return result.map(row => ({
      user: row.user || Object.values(row)[0] || '',
      host: row.host || Object.values(row)[1] || '',
    }));
  } catch (err) {
    console.error('listUsers error:', err);
    return [];
  }
}

// Create user
export async function createUser(
  username: string,
  password: string,
  host: string = 'localhost'
): Promise<void> {
  if (!db) throw new Error('Not connected');

  // Validate username
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(username)) {
    throw new Error('Invalid username');
  }

  await db.execute(`CREATE USER '${username}'@'${host}' IDENTIFIED BY '${password}'`);
}

// Drop user
export async function dropUser(username: string, host: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  // Prevent dropping root
  if (username.toLowerCase() === 'root') {
    throw new Error('Cannot drop root user');
  }

  await db.execute(`DROP USER '${username}'@'${host}'`);
}

// Change password
export async function changePassword(
  username: string,
  host: string,
  newPassword: string
): Promise<void> {
  if (!db) throw new Error('Not connected');

  await db.execute(`ALTER USER '${username}'@'${host}' IDENTIFIED BY '${newPassword}'`);
}

// Grant all privileges on database to user
export async function grantPrivileges(
  username: string,
  host: string,
  database: string
): Promise<void> {
  if (!db) throw new Error('Not connected');

  await db.execute(`GRANT ALL PRIVILEGES ON \`${database}\`.* TO '${username}'@'${host}'`);
  await db.execute('FLUSH PRIVILEGES');
}

// Revoke all privileges
export async function revokePrivileges(
  username: string,
  host: string,
  database: string
): Promise<void> {
  if (!db) throw new Error('Not connected');

  await db.execute(`REVOKE ALL PRIVILEGES ON \`${database}\`.* FROM '${username}'@'${host}'`);
  await db.execute('FLUSH PRIVILEGES');
}

// Get user privileges
export async function getUserPrivileges(username: string, host: string): Promise<string[]> {
  if (!db) throw new Error('Not connected');

  const result = await db.select<Record<string, string>[]>(`SHOW GRANTS FOR '${username}'@'${host}'`);

  return result.map(row => Object.values(row)[0]);
}

// Helper: Format bytes
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}
