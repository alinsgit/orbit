import Database from '@tauri-apps/plugin-sql';

// Connection state
let db: Database | null = null;
let currentEngine: 'mariadb' | 'postgresql' | 'mongodb' | null = null;

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
  engine?: 'mariadb' | 'postgresql' | 'mongodb';
  host: string;
  port: number;
  user: string;
  password?: string;
}

// Connect to Database
export async function dbConnect(config: DbConnectionConfig): Promise<void> {
  const engine = config.engine || 'mariadb';
  currentEngine = engine;
  
  if (config.host !== '127.0.0.1' && config.host !== 'localhost') {
    throw new Error('Only localhost connections are allowed');
  }

  const passStr = config.password ? `:${encodeURIComponent(config.password)}` : '';
  let connectionString = '';
  
  if (engine === 'postgresql') {
    connectionString = `postgres://${encodeURIComponent(config.user)}${passStr}@${config.host}:${config.port}/postgres`;
  } else if (engine === 'mariadb') {
    connectionString = `mysql://${encodeURIComponent(config.user)}${passStr}@${config.host}:${config.port}/mysql`;
  } else {
    throw new Error('Unsupported engine for SQL connection (use MongoDB native methods via backend)');
  }

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
    currentEngine = null;
  }
}

// Check connection
export function isConnected(): boolean {
  return db !== null;
}

// Get currently connected engine
export function getConnectedEngine(): 'mariadb' | 'postgresql' | 'mongodb' | null {
  return currentEngine;
}

const getVal = (result: Record<string, any>[]) => {
  if (!result[0]) return '';
  return result[0].Value || result[0].value || Object.values(result[0])[1] || Object.values(result[0])[0] || '';
};

// Get server info
export async function getServerInfo(): Promise<ServerInfo> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    const versionResult = await db.select<any[]>("SELECT version() as version");
    const versionParts = getVal(versionResult).split(' ');
    const version = versionParts[1] || getVal(versionResult); 
    
    let uptime = 0;
    try {
      const uptimeResult = await db.select<any[]>("SELECT extract(epoch from current_timestamp - pg_postmaster_start_time()) as uptime");
      uptime = parseInt(getVal(uptimeResult));
    } catch { }

    let connections = 0;
    try {
      const connectionsResult = await db.select<any[]>("SELECT sum(numbackends) as connections FROM pg_stat_database");
      connections = parseInt(getVal(connectionsResult));
    } catch { }
    
    return { version, uptime, connections };
  } else {
    const versionResult = await db.select<any[]>("SHOW VARIABLES LIKE 'version'");
    const uptimeResult = await db.select<any[]>("SHOW STATUS LIKE 'Uptime'");
    const connectionsResult = await db.select<any[]>("SHOW STATUS LIKE 'Threads_connected'");

    return {
      version: getVal(versionResult) || 'Unknown',
      uptime: parseInt(getVal(uptimeResult) || '0'),
      connections: parseInt(getVal(connectionsResult) || '0'),
    };
  }
}

// List databases
export async function listDatabases(): Promise<DatabaseInfo[]> {
  if (!db) throw new Error('Not connected');

  let result: any[] = [];
  if (currentEngine === 'postgresql') {
    result = await db.select<any[]>("SELECT datname as Database FROM pg_database WHERE datistemplate = false");
  } else {
    result = await db.select<any[]>('SHOW DATABASES');
  }

  return result.map(row => ({
    name: row.Database || row.datname || Object.values(row)[0] || 'unknown',
  }));
}

// Create database
export async function createDatabase(name: string, charset: string = 'utf8mb4', collation: string = 'utf8mb4_unicode_ci'): Promise<void> {
  if (!db) throw new Error('Not connected');
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(name)) throw new Error('Invalid database name');

  if (currentEngine === 'postgresql') {
    await db.execute(`CREATE DATABASE "${name}" ENCODING 'UTF8'`);
  } else {
    await db.execute(`CREATE DATABASE \`${name}\` CHARACTER SET ${charset} COLLATE ${collation}`);
  }
}

// Drop database
export async function dropDatabase(name: string): Promise<void> {
  if (!db) throw new Error('Not connected');
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(name)) throw new Error('Invalid database name');

  if (currentEngine === 'postgresql') {
    const systemDbs = ['postgres'];
    if (systemDbs.includes(name.toLowerCase())) throw new Error('Cannot drop system database');
    await db.execute(`DROP DATABASE "${name}"`);
  } else {
    const systemDbs = ['mysql', 'information_schema', 'performance_schema', 'sys'];
    if (systemDbs.includes(name.toLowerCase())) throw new Error('Cannot drop system database');
    await db.execute(`DROP DATABASE \`${name}\``);
  }
}

// List users
export async function listUsers(): Promise<UserInfo[]> {
  if (!db) throw new Error('Not connected');

  try {
    if (currentEngine === 'postgresql') {
      const result = await db.select<any[]>('SELECT usename as user, \'localhost\' as host FROM pg_user');
      return result.map((row: any) => ({
        user: row.user || row.usename || Object.values(row)[0] || '',
        host: row.host || Object.values(row)[1] || '',
      }));
    } else {
      const result = await db.select<any[]>('SELECT CAST(User AS CHAR) as user, CAST(Host AS CHAR) as host FROM mysql.user');
      return result.map((row: any) => ({
        user: row.user || Object.values(row)[0] || '',
        host: row.host || Object.values(row)[1] || '',
      }));
    }
  } catch (err) {
    console.error('listUsers error:', err);
    return [];
  }
}

// Create user
export async function createUser(username: string, password: string, host: string = 'localhost'): Promise<void> {
  if (!db) throw new Error('Not connected');
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(username)) throw new Error('Invalid username');

  if (currentEngine === 'postgresql') {
    await db.execute(`CREATE ROLE "${username}" WITH LOGIN PASSWORD '${password}'`);
  } else {
    await db.execute(`CREATE USER '${username}'@'${host}' IDENTIFIED BY '${password}'`);
  }
}

// Drop user
export async function dropUser(username: string, host: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    if (username.toLowerCase() === 'postgres') throw new Error('Cannot drop root user');
    await db.execute(`DROP ROLE "${username}"`);
  } else {
    if (username.toLowerCase() === 'root') throw new Error('Cannot drop root user');
    await db.execute(`DROP USER '${username}'@'${host}'`);
  }
}

// Change password
export async function changePassword(username: string, host: string, newPassword: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    await db.execute(`ALTER ROLE "${username}" WITH PASSWORD '${newPassword}'`);
  } else {
    await db.execute(`ALTER USER '${username}'@'${host}' IDENTIFIED BY '${newPassword}'`);
  }
}

// Grant privileges
export async function grantPrivileges(username: string, host: string, database: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    await db.execute(`GRANT ALL PRIVILEGES ON DATABASE "${database}" TO "${username}"`);
  } else {
    await db.execute(`GRANT ALL PRIVILEGES ON \`${database}\`.* TO '${username}'@'${host}'`);
    await db.execute('FLUSH PRIVILEGES');
  }
}

// Revoke privileges
export async function revokePrivileges(username: string, host: string, database: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    await db.execute(`REVOKE ALL PRIVILEGES ON DATABASE "${database}" FROM "${username}"`);
  } else {
    await db.execute(`REVOKE ALL PRIVILEGES ON \`${database}\`.* FROM '${username}'@'${host}'`);
    await db.execute('FLUSH PRIVILEGES');
  }
}

// Get user privileges
export async function getUserPrivileges(username: string, host: string): Promise<string[]> {
  if (!db) return [];

  if (currentEngine === 'postgresql') {
    return ['postgres_all'];
  } else {
    const result = await db.select<Record<string, string>[]>(`SHOW GRANTS FOR '${username}'@'${host}'`);
    return result.map(row => Object.values(row)[0]);
  }
}

export async function getDatabaseCharset(name: string): Promise<{ charset: string; collation: string }> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    const result = await db.select<any[]>(`SELECT encoding, datcollate FROM pg_database WHERE datname = '${name}'`);
    if (!result[0]) throw new Error('Database not found');
    return {
      charset: 'UTF8',
      collation: result[0].datcollate || 'en_US.utf8',
    };
  } else {
    const result = await db.select<Record<string, string>[]>(
      `SELECT DEFAULT_CHARACTER_SET_NAME as charset, DEFAULT_COLLATION_NAME as collation 
       FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = '${name}'`
    );
    if (!result[0]) throw new Error('Database not found');
    return {
      charset: result[0].charset || 'utf8mb4',
      collation: result[0].collation || 'utf8mb4_unicode_ci',
    };
  }
}

export async function alterDatabaseCharset(name: string, charset: string, collation: string): Promise<void> {
  if (!db) throw new Error('Not connected');

  if (currentEngine === 'postgresql') {
    throw new Error('PostgreSQL does not support altering database encoding after creation. Recreate the database with the desired encoding instead.');
  } else {
    const systemDbs = ['mysql', 'information_schema', 'performance_schema', 'sys'];
    if (systemDbs.includes(name.toLowerCase())) {
      throw new Error('Cannot alter system database');
    }
    await db.execute(`ALTER DATABASE \`${name}\` CHARACTER SET ${charset} COLLATE ${collation}`);
  }
}

export async function getDatabaseUsers(database: string): Promise<UserInfo[]> {
  if (!db) throw new Error('Not connected');

  const users = await listUsers();
  if (currentEngine === 'postgresql') {
    return users; // Overly permissive for pg local dev, but works to unblock Native UI rapidly
  }

  const dbUsers: UserInfo[] = [];

  for (const user of users) {
    try {
      const grants = await getUserPrivileges(user.user, user.host);
      const hasAccess = grants.some(g => {
        const upper = g.toUpperCase();
        return upper.includes(`\`${database}\``) || upper.includes('ON *.*');
      });
      if (hasAccess) {
        dbUsers.push(user);
      }
    } catch { }
  }

  return dbUsers;
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}
