import { useState, useEffect } from 'react';
import {
  Database,
  Users,
  Plus,
  Trash2,
  Key,
  RefreshCw,
  Server,
  HardDrive,
  Shield,
  AlertCircle,
  CheckCircle,
  LogOut,
  Eye,
  EyeOff,
  Download,
  Upload,
  Pencil,
} from 'lucide-react';
import { save, open, ask } from '@tauri-apps/plugin-dialog';
import {
  dbConnect,
  dbDisconnect,
  isConnected,
  getConnectedEngine,
  getServerInfo,
  listDatabases,
  createDatabase,
  dropDatabase,
  listUsers,
  createUser,
  dropUser,
  changePassword,
  grantPrivileges,
  revokePrivileges,
  getDatabaseCharset,
  alterDatabaseCharset,
  getDatabaseUsers,
  type DatabaseInfo,
  type UserInfo,
  type ServerInfo,
  type DbConnectionConfig,
} from '../../lib/db-api';
import {
  exportDatabase,
  exportAllDatabases,
  importSql,
} from '../../lib/api';

type Tab = 'databases' | 'users';

interface DialogState {
  type: 'createDb' | 'createUser' | 'changePassword' | 'grantPrivileges' | 'editDb' | null;
  data?: UserInfo;
  dbName?: string;
}

interface EditDbState {
  charset: string;
  collation: string;
  authorizedUsers: string[]; // "user@host" keys
  originalUsers: string[];
}

const CHARSET_COLLATIONS: Record<string, string[]> = {
  utf8mb4: ['utf8mb4_unicode_ci', 'utf8mb4_general_ci', 'utf8mb4_bin', 'utf8mb4_turkish_ci'],
  utf8: ['utf8_unicode_ci', 'utf8_general_ci', 'utf8_bin', 'utf8_turkish_ci'],
  latin1: ['latin1_swedish_ci', 'latin1_general_ci', 'latin1_bin'],
  latin5: ['latin5_turkish_ci', 'latin5_bin'],
};

interface NativeDatabaseManagerProps {
  dbEngine?: 'mariadb' | 'postgresql' | 'mongodb';
}

export default function NativeDatabaseManager({ dbEngine = 'mariadb' }: NativeDatabaseManagerProps) {
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>('databases');

  // Connection form
  const [config, setConfig] = useState<DbConnectionConfig>({
    engine: dbEngine,
    host: '127.0.0.1',
    port: dbEngine === 'postgresql' ? 5432 : 3306,
    user: dbEngine === 'postgresql' ? 'postgres' : 'root',
    password: '',
  });

  useEffect(() => {
    // Disconnect when engine changes to prevent cross-engine data leaks
    if (connected) {
      dbDisconnect().catch(() => {});
      setConnected(false);
      setServerInfo(null);
      setDatabases([]);
      setUsers([]);
    }
    setConfig(c => ({
      ...c,
      engine: dbEngine,
      port: dbEngine === 'postgresql' ? 5432 : 3306,
      user: dbEngine === 'postgresql' ? 'postgres' : 'root',
    }));
  }, [dbEngine]);
  const [showPassword, setShowPassword] = useState(false);

  // Data
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const [databases, setDatabases] = useState<DatabaseInfo[]>([]);
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [backupLoading, setBackupLoading] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  // Dialogs
  const [dialog, setDialog] = useState<DialogState>({ type: null });
  const [dialogLoading, setDialogLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [editDb, setEditDb] = useState<EditDbState | null>(null);

  // Dialog form states
  const [newDbName, setNewDbName] = useState('');
  const [newDbCharset, setNewDbCharset] = useState('utf8mb4');
  const [newUsername, setNewUsername] = useState('');
  const [newUserPassword, setNewUserPassword] = useState('');
  const [newUserHost, setNewUserHost] = useState('localhost');
  const [newPassword, setNewPassword] = useState('');
  const [selectedDatabase, setSelectedDatabase] = useState('');

  useEffect(() => {
    const alreadyConnected = isConnected();
    const connectedEngine = getConnectedEngine();
    // Only show as connected if the engine matches
    if (alreadyConnected && connectedEngine === dbEngine) {
      setConnected(true);
      loadData();
    } else if (alreadyConnected && connectedEngine !== dbEngine) {
      // Connected to a different engine — don't show this engine's data
      setConnected(false);
    }
  }, []);

  const handleConnect = async () => {
    try {
      setConnecting(true);
      setError(null);
      await dbConnect(config);
      setConnected(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed');
      setConnecting(false);
      return;
    }

    setConnecting(false);
    // Load data after state is updated
    await loadData();
  };

  const handleDisconnect = async () => {
    await dbDisconnect();
    setConnected(false);
    setServerInfo(null);
    setDatabases([]);
    setUsers([]);
  };

  const loadData = async () => {
    setLoading(true);
    setError(null);

    // Load each separately so one failure doesn't block others
    try {
      const info = await getServerInfo();
      setServerInfo(info);
    } catch { /* server info not critical */ }

    try {
      const dbs = await listDatabases();
      setDatabases(dbs);
    } catch { /* will show empty list */ }

    try {
      const userList = await listUsers();
      setUsers(userList);
    } catch { /* will show empty list */ }

    setLoading(false);
  };

  const handleCreateDatabase = async () => {
    try {
      setDialogLoading(true);
      await createDatabase(newDbName, newDbCharset);
      setDialog({ type: null });
      setNewDbName('');
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create database');
    } finally {
      setDialogLoading(false);
    }
  };

  const handleDropDatabase = async (name: string) => {
    const confirmed = await ask(`Are you sure you want to delete the database '${name}'? This action cannot be undone.`, { title: 'Confirm Delete', kind: 'warning' });
    if (!confirmed) return;
    try {
      setActionLoading(`dropDb-${name}`);
      await dropDatabase(name);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to drop database');
    } finally {
      setActionLoading(null);
    }
  };

  const handleCreateUser = async () => {
    try {
      setDialogLoading(true);
      await createUser(newUsername, newUserPassword, newUserHost);
      setDialog({ type: null });
      setNewUsername('');
      setNewUserPassword('');
      setNewUserHost('localhost');
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create user');
    } finally {
      setDialogLoading(false);
    }
  };

  const handleDropUser = async (user: UserInfo) => {
    const confirmed = await ask(`Are you sure you want to delete the user '${user.user}'?`, { title: 'Confirm Delete', kind: 'warning' });
    if (!confirmed) return;
    try {
      setActionLoading(`dropUser-${user.user}@${user.host}`);
      await dropUser(user.user, user.host);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to drop user');
    } finally {
      setActionLoading(null);
    }
  };

  const handleChangePassword = async () => {
    if (!dialog.data) return;
    try {
      setDialogLoading(true);
      await changePassword(dialog.data.user, dialog.data.host, newPassword);
      setDialog({ type: null });
      setNewPassword('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to change password');
    } finally {
      setDialogLoading(false);
    }
  };

  const handleGrantPrivileges = async () => {
    if (!dialog.data || !selectedDatabase) return;
    try {
      setDialogLoading(true);
      await grantPrivileges(dialog.data.user, dialog.data.host, selectedDatabase);
      setDialog({ type: null });
      setSelectedDatabase('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to grant privileges');
    } finally {
      setDialogLoading(false);
    }
  };

  const formatUptime = (seconds: number): string => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${mins}m`;
    return `${mins}m`;
  };

  const showSuccess = (msg: string) => {
    setSuccessMsg(msg);
    setTimeout(() => setSuccessMsg(null), 4000);
  };

  const handleExportDatabase = async (dbName: string) => {
    try {
      const path = await save({
        defaultPath: `${dbName}_backup.sql`,
        filters: [{ name: 'SQL Files', extensions: ['sql'] }],
      });
      if (!path) return;
      setBackupLoading(`export-${dbName}`);
      const result = await exportDatabase(dbName, path);
      showSuccess(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Export failed');
    } finally {
      setBackupLoading(null);
    }
  };

  const handleExportAll = async () => {
    try {
      const now = new Date().toISOString().slice(0, 10);
      const path = await save({
        defaultPath: `all_databases_${now}.sql`,
        filters: [{ name: 'SQL Files', extensions: ['sql'] }],
      });
      if (!path) return;
      setBackupLoading('export-all');
      const result = await exportAllDatabases(path);
      showSuccess(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Export all failed');
    } finally {
      setBackupLoading(null);
    }
  };

  const handleImportSql = async (dbName: string) => {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: 'SQL Files', extensions: ['sql'] }],
      });
      if (!path) return;
      setBackupLoading(`import-${dbName}`);
      const result = await importSql(dbName, path as string);
      showSuccess(result);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Import failed');
    } finally {
      setBackupLoading(null);
    }
  };

  const handleOpenEditDb = async (dbName: string) => {
    try {
      setDialogLoading(true);
      setDialog({ type: 'editDb', dbName });
      const [charsetInfo, dbUsers] = await Promise.all([
        getDatabaseCharset(dbName),
        getDatabaseUsers(dbName),
      ]);
      const userKeys = dbUsers.map(u => `${u.user}@${u.host}`);
      setEditDb({
        charset: charsetInfo.charset,
        collation: charsetInfo.collation,
        authorizedUsers: userKeys,
        originalUsers: [...userKeys],
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load database info');
      setDialog({ type: null });
    } finally {
      setDialogLoading(false);
    }
  };

  const handleSaveEditDb = async () => {
    if (!dialog.dbName || !editDb) return;
    try {
      setDialogLoading(true);
      // 1. Update charset/collation
      const currentCharset = await getDatabaseCharset(dialog.dbName);
      if (currentCharset.charset !== editDb.charset || currentCharset.collation !== editDb.collation) {
        await alterDatabaseCharset(dialog.dbName, editDb.charset, editDb.collation);
      }
      // 2. Sync user privileges
      const added = editDb.authorizedUsers.filter(u => !editDb.originalUsers.includes(u));
      const removed = editDb.originalUsers.filter(u => !editDb.authorizedUsers.includes(u));
      for (const key of added) {
        const [user, host] = key.split('@');
        await grantPrivileges(user, host, dialog.dbName);
      }
      for (const key of removed) {
        const [user, host] = key.split('@');
        await revokePrivileges(user, host, dialog.dbName);
      }
      showSuccess(`Database '${dialog.dbName}' updated`);
      setDialog({ type: null });
      setEditDb(null);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update database');
    } finally {
      setDialogLoading(false);
    }
  };

  // Not connected - show login form
  if (!connected) {
    return (
      <div className="p-6 h-full overflow-y-auto">
        <div className="max-w-md mx-auto mt-12">
          <div className="flex items-center gap-3 mb-6">
            <Database className="w-8 h-8 text-emerald-500" />
            <div>
              <h2 className="text-xl font-semibold text-content">Database Manager</h2>
              <p className="text-sm text-content-muted">Connect to {dbEngine === 'postgresql' ? 'PostgreSQL' : 'MariaDB'}</p>
            </div>
          </div>

          {error && (
            <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg flex items-center gap-2 text-red-400">
              <AlertCircle className="w-5 h-5 flex-shrink-0" />
              <span className="text-sm">{error}</span>
            </div>
          )}

          <div className="bg-surface-raised rounded-xl p-6 space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm text-content-muted mb-1">Host</label>
                <input
                  type="text"
                  value={config.host}
                  onChange={e => setConfig({ ...config, host: e.target.value })}
                  className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                  placeholder="127.0.0.1"
                />
              </div>
              <div>
                <label className="block text-sm text-content-muted mb-1">Port</label>
                <input
                  type="number"
                  value={config.port}
                  onChange={e => setConfig({ ...config, port: parseInt(e.target.value) })}
                  className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                />
              </div>
            </div>

            <div>
              <label className="block text-sm text-content-muted mb-1">Username</label>
              <input
                type="text"
                value={config.user}
                onChange={e => setConfig({ ...config, user: e.target.value })}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                placeholder="root"
              />
            </div>

            <div>
              <label className="block text-sm text-content-muted mb-1">Password</label>
              <div className="relative">
                <input
                  type={showPassword ? 'text' : 'password'}
                  value={config.password}
                  onChange={e => setConfig({ ...config, password: e.target.value })}
                  onKeyDown={e => e.key === 'Enter' && handleConnect()}
                  className="w-full px-3 py-2 pr-10 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                  placeholder="Enter password"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-content-muted hover:text-content"
                >
                  {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
            </div>

            <button
              onClick={handleConnect}
              disabled={connecting}
              className="w-full py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-white font-medium transition-colors flex items-center justify-center gap-2"
            >
              {connecting ? (
                <>
                  <RefreshCw className="w-4 h-4 animate-spin" />
                  Connecting...
                </>
              ) : (
                <>
                  <Server className="w-4 h-4" />
                  Connect
                </>
              )}
            </button>
          </div>

          <p className="text-xs text-content-muted text-center mt-4">
            Only localhost connections are allowed for security
          </p>
        </div>
      </div>
    );
  }

  // Connected - show manager
  return (
    <div className="p-6 h-full overflow-y-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <Database className="w-8 h-8 text-emerald-500" />
          <div>
            <h2 className="text-xl font-semibold text-content">Database Manager</h2>
            <div className="flex items-center gap-2 text-sm text-content-muted">
              <CheckCircle className="w-4 h-4 text-emerald-500" />
              Connected to {config.host}:{config.port}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={loadData}
            disabled={loading}
            className="p-2 hover:bg-hover rounded-lg transition-colors"
            title="Refresh"
          >
            <RefreshCw className={`w-5 h-5 ${loading ? 'animate-spin' : ''}`} />
          </button>
          <button
            onClick={handleDisconnect}
            className="flex items-center gap-2 px-3 py-2 bg-surface-raised hover:bg-hover rounded-lg text-sm transition-colors"
          >
            <LogOut className="w-4 h-4" />
            Disconnect
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg flex items-center gap-2 text-red-400">
          <AlertCircle className="w-5 h-5 flex-shrink-0" />
          <span className="text-sm">{error}</span>
          <button onClick={() => setError(null)} className="ml-auto text-red-400 hover:text-red-300">
            &times;
          </button>
        </div>
      )}

      {successMsg && (
        <div className="mb-4 p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-lg flex items-center gap-2 text-emerald-400">
          <CheckCircle className="w-5 h-5 flex-shrink-0" />
          <span className="text-sm">{successMsg}</span>
          <button onClick={() => setSuccessMsg(null)} className="ml-auto text-emerald-400 hover:text-emerald-300">
            &times;
          </button>
        </div>
      )}

      {/* Server Info */}
      {serverInfo && (
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-surface-raised rounded-xl p-4">
            <div className="flex items-center gap-2 text-content-muted text-sm mb-1">
              <Server className="w-4 h-4" />
              Version
            </div>
            <p className="text-content font-medium">{serverInfo.version}</p>
          </div>
          <div className="bg-surface-raised rounded-xl p-4">
            <div className="flex items-center gap-2 text-content-muted text-sm mb-1">
              <RefreshCw className="w-4 h-4" />
              Uptime
            </div>
            <p className="text-content font-medium">{formatUptime(serverInfo.uptime)}</p>
          </div>
          <div className="bg-surface-raised rounded-xl p-4">
            <div className="flex items-center gap-2 text-content-muted text-sm mb-1">
              <Users className="w-4 h-4" />
              Connections
            </div>
            <p className="text-content font-medium">{serverInfo.connections}</p>
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-2 mb-4 border-b border-edge-subtle">
        <button
          onClick={() => setActiveTab('databases')}
          className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
            activeTab === 'databases'
              ? 'border-emerald-500 text-emerald-500'
              : 'border-transparent text-content-muted hover:text-content'
          }`}
        >
          <HardDrive className="w-4 h-4 inline mr-2" />
          Databases ({databases.length})
        </button>
        <button
          onClick={() => setActiveTab('users')}
          className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
            activeTab === 'users'
              ? 'border-emerald-500 text-emerald-500'
              : 'border-transparent text-content-muted hover:text-content'
          }`}
        >
          <Users className="w-4 h-4 inline mr-2" />
          Users ({users.length})
        </button>
      </div>

      {/* Databases Tab */}
      {activeTab === 'databases' && (
        <div>
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-sm font-medium text-content-secondary">Databases</h3>
              <div className="flex gap-2">
                {dbEngine === 'mariadb' && (
                  <>
                    <button
                      onClick={handleExportAll}
                      disabled={backupLoading !== null || !databases.length}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-surface-raised hover:bg-surface text-content-secondary hover:text-content rounded-lg transition-colors border border-edge disabled:opacity-50"
                      title="Export all databases to a single SQL file"
                    >
                      {backupLoading === 'export-all' ? (
                        <RefreshCw className="w-4 h-4 animate-spin" />
                      ) : (
                        <Download className="w-4 h-4" />
                      )}
                      Export All
                    </button>
                    <button
                      onClick={() => {
                        setSelectedDatabase(''); // Clear selection so they must pick
                        handleImportSql('');
                      }}
                      disabled={backupLoading !== null}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-surface-raised hover:bg-surface text-content-secondary hover:text-content rounded-lg transition-colors border border-edge disabled:opacity-50"
                      title="Import an SQL file into a database"
                    >
                      {backupLoading === 'import' ? (
                        <RefreshCw className="w-4 h-4 animate-spin" />
                      ) : (
                        <Upload className="w-4 h-4" />
                      )}
                      Import
                    </button>
                  </>
                )}
                <button
                  onClick={() => setDialog({ type: 'createDb' })}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-emerald-600 hover:bg-emerald-500 rounded-lg text-white transition-colors"
                >
                  <Plus className="w-4 h-4" />
                  Create Database
                </button>
              </div>
          </div>

          <div className="space-y-2">
            {databases.map(db => (
              <div
                key={db.name}
                className="group flex items-center justify-between p-3 bg-surface-raised rounded-lg hover:bg-hover transition-colors"
              >
                <div className="flex items-center gap-3">
                  <HardDrive className="w-5 h-5 text-content-muted" />
                  <div>
                    <p className="font-medium text-content">{db.name}</p>
                    <p className="text-xs text-content-muted">
                      {db.tables !== undefined ? `${db.tables} tables` : ''}
                      {db.size ? ` • ${db.size}` : ''}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  {!['mysql', 'information_schema', 'performance_schema', 'sys', 'postgres'].includes(db.name.toLowerCase()) && (
                    <div className="flex gap-1 justify-end opacity-0 group-hover:opacity-100 transition-opacity">
                      {dbEngine === 'mariadb' && (
                        <button
                          onClick={() => handleExportDatabase(db.name)}
                          disabled={backupLoading === `export-${db.name}`}
                          className="p-1.5 text-content-muted hover:text-content hover:bg-surface-raised rounded-md transition-colors"
                          title="Export Database"
                        >
                          {backupLoading === `export-${db.name}` ? (
                            <RefreshCw className="w-4 h-4 animate-spin" />
                          ) : (
                            <Download className="w-4 h-4" />
                          )}
                        </button>
                      )}
                      
                      <button
                        onClick={() => handleOpenEditDb(db.name)}
                        className="p-1.5 text-content-muted hover:text-content hover:bg-surface-raised rounded-md transition-colors"
                        title="Database Settings (Charset/Collation/Users)"
                      >
                        <Pencil className="w-4 h-4" />
                      </button>

                      {dbEngine === 'mariadb' && (
                        <button
                          onClick={() => handleImportSql(db.name)}
                          disabled={backupLoading === `import-${db.name}`}
                          className="p-1.5 text-content-muted hover:text-content hover:bg-surface-raised rounded-md transition-colors"
                          title="Import SQL file into this database"
                        >
                          {backupLoading === `import-${db.name}` ? (
                            <RefreshCw className="w-4 h-4 animate-spin" />
                          ) : (
                            <Upload className="w-4 h-4" />
                          )}
                        </button>
                      )}
                      
                      <button
                        onClick={() => handleDropDatabase(db.name)}
                        disabled={actionLoading === `dropDb-${db.name}`}
                        className="p-2 text-red-400 hover:bg-red-500/10 disabled:opacity-50 rounded-lg transition-colors"
                        title="Delete database"
                      >
                        {actionLoading === `dropDb-${db.name}` ? (
                          <RefreshCw className="w-4 h-4 animate-spin" />
                        ) : (
                          <Trash2 className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Users Tab */}
      {activeTab === 'users' && (
        <div>
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-sm font-medium text-content-secondary">Users</h3>
            <button
              onClick={() => setDialog({ type: 'createUser' })}
              className="flex items-center gap-2 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm transition-colors"
            >
              <Plus className="w-4 h-4" />
              Create User
            </button>
          </div>

          <div className="space-y-2">
            {users.map(user => (
              <div
                key={`${user.user}@${user.host}`}
                className="flex items-center justify-between p-3 bg-surface-raised rounded-lg hover:bg-hover transition-colors"
              >
                <div className="flex items-center gap-3">
                  <Users className="w-5 h-5 text-content-muted" />
                  <div>
                    <p className="font-medium text-content">{user.user}</p>
                    <p className="text-xs text-content-muted">@{user.host}</p>
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => setDialog({ type: 'changePassword', data: user })}
                    className="p-2 text-content-muted hover:bg-hover rounded-lg transition-colors"
                    title="Change password"
                  >
                    <Key className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => setDialog({ type: 'grantPrivileges', data: user })}
                    className="p-2 text-content-muted hover:bg-hover rounded-lg transition-colors"
                    title="Grant privileges"
                  >
                    <Shield className="w-4 h-4" />
                  </button>
                  {user.user.toLowerCase() !== 'root' && user.user.toLowerCase() !== 'postgres' && (
                    <button
                      onClick={() => handleDropUser(user)}
                      disabled={actionLoading === `dropUser-${user.user}@${user.host}`}
                      className="p-2 text-red-400 hover:bg-red-500/10 disabled:opacity-50 rounded-lg transition-colors"
                      title="Delete user"
                    >
                      {actionLoading === `dropUser-${user.user}@${user.host}` ? (
                        <RefreshCw className="w-4 h-4 animate-spin" />
                      ) : (
                        <Trash2 className="w-4 h-4" />
                      )}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Create Database Dialog */}
      {dialog.type === 'createDb' && (
        <Dialog title="Create Database" onClose={() => setDialog({ type: null })}>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-content-muted mb-1">Database Name</label>
              <input
                type="text"
                value={newDbName}
                onChange={e => setNewDbName(e.target.value)}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                placeholder="my_database"
                autoFocus
              />
            </div>
            {dbEngine === 'mariadb' && (
              <div>
                <label className="block text-sm text-content-muted mb-1">Character Set</label>
                <select
                  value={newDbCharset}
                  onChange={e => setNewDbCharset(e.target.value)}
                  className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                >
                  <option value="utf8mb4">utf8mb4 (Recommended)</option>
                  <option value="utf8">utf8</option>
                  <option value="latin1">latin1</option>
                </select>
              </div>
            )}
            {dbEngine === 'postgresql' && (
              <p className="text-xs text-content-muted">PostgreSQL databases use UTF8 encoding by default.</p>
            )}
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setDialog({ type: null })}
                className="px-4 py-2 bg-surface-raised hover:bg-hover rounded-lg text-sm transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateDatabase}
                disabled={!newDbName || dialogLoading}
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
              >
                {dialogLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  'Create'
                )}
              </button>
            </div>
          </div>
        </Dialog>
      )}

      {/* Create User Dialog */}
      {dialog.type === 'createUser' && (
        <Dialog title="Create User" onClose={() => setDialog({ type: null })}>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-content-muted mb-1">Username</label>
              <input
                type="text"
                value={newUsername}
                onChange={e => setNewUsername(e.target.value)}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                placeholder="new_user"
                autoFocus
              />
            </div>
            <div>
              <label className="block text-sm text-content-muted mb-1">Password</label>
              <input
                type="password"
                value={newUserPassword}
                onChange={e => setNewUserPassword(e.target.value)}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                placeholder="Enter password"
              />
            </div>
            <div>
              <label className="block text-sm text-content-muted mb-1">Host</label>
              <select
                value={newUserHost}
                onChange={e => setNewUserHost(e.target.value)}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
              >
                <option value="localhost">localhost</option>
                <option value="127.0.0.1">127.0.0.1</option>
                <option value="%">% (Any host)</option>
              </select>
            </div>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setDialog({ type: null })}
                className="px-4 py-2 bg-surface-raised hover:bg-hover rounded-lg text-sm transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateUser}
                disabled={!newUsername || !newUserPassword || dialogLoading}
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
              >
                {dialogLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  'Create'
                )}
              </button>
            </div>
          </div>
        </Dialog>
      )}

      {/* Change Password Dialog */}
      {dialog.type === 'changePassword' && dialog.data && (
        <Dialog title={`Change Password for ${dialog.data.user}`} onClose={() => setDialog({ type: null })}>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-content-muted mb-1">New Password</label>
              <input
                type="password"
                value={newPassword}
                onChange={e => setNewPassword(e.target.value)}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                placeholder="Enter new password"
                autoFocus
              />
            </div>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setDialog({ type: null })}
                className="px-4 py-2 bg-surface-raised hover:bg-hover rounded-lg text-sm transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleChangePassword}
                disabled={!newPassword || dialogLoading}
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
              >
                {dialogLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    Changing...
                  </>
                ) : (
                  'Change Password'
                )}
              </button>
            </div>
          </div>
        </Dialog>
      )}

      {/* Grant Privileges Dialog */}
      {dialog.type === 'grantPrivileges' && dialog.data && (
        <Dialog title={`Grant Privileges to ${dialog.data.user}`} onClose={() => setDialog({ type: null })}>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-content-muted mb-1">Select Database</label>
              <select
                value={selectedDatabase}
                onChange={e => setSelectedDatabase(e.target.value)}
                className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
              >
                <option value="">Select a database...</option>
                {databases
                  .filter(db => {
                    const systemDbs = dbEngine === 'postgresql'
                      ? ['postgres', 'template0', 'template1']
                      : ['mysql', 'information_schema', 'performance_schema', 'sys'];
                    return !systemDbs.includes(db.name.toLowerCase());
                  })
                  .map(db => (
                    <option key={db.name} value={db.name}>{db.name}</option>
                  ))
                }
              </select>
            </div>
            <p className="text-xs text-content-muted">
              This will grant ALL PRIVILEGES on the selected database to {dialog.data.user}@{dialog.data.host}
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setDialog({ type: null })}
                className="px-4 py-2 bg-surface-raised hover:bg-hover rounded-lg text-sm transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleGrantPrivileges}
                disabled={!selectedDatabase || dialogLoading}
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
              >
                {dialogLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    Granting...
                  </>
                ) : (
                  'Grant Privileges'
                )}
              </button>
            </div>
          </div>
        </Dialog>
      )}

      {/* Edit Database Dialog */}
      {dialog.type === 'editDb' && dialog.dbName && (
        <Dialog title={`Edit: ${dialog.dbName}`} onClose={() => { setDialog({ type: null }); setEditDb(null); }}>
          {!editDb ? (
            <div className="flex items-center justify-center py-8">
              <RefreshCw className="w-5 h-5 animate-spin text-content-muted" />
              <span className="ml-2 text-sm text-content-muted">Loading...</span>
            </div>
          ) : (
            <div className="space-y-5">
              {/* Charset & Collation — MariaDB only */}
              {dbEngine === 'mariadb' && (
                <>
                  <div>
                    <label className="block text-sm text-content-muted mb-1">Character Set</label>
                    <select
                      value={editDb.charset}
                      onChange={e => {
                        const newCharset = e.target.value;
                        const collations = CHARSET_COLLATIONS[newCharset] || [];
                        setEditDb({ ...editDb, charset: newCharset, collation: collations[0] || '' });
                      }}
                      className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                    >
                      {Object.keys(CHARSET_COLLATIONS).map(cs => (
                        <option key={cs} value={cs}>{cs}</option>
                      ))}
                    </select>
                  </div>

                  <div>
                    <label className="block text-sm text-content-muted mb-1">Collation</label>
                    <select
                      value={editDb.collation}
                      onChange={e => setEditDb({ ...editDb, collation: e.target.value })}
                      className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg text-content text-sm focus:outline-none focus:border-emerald-500"
                    >
                      {(CHARSET_COLLATIONS[editDb.charset] || []).map(col => (
                        <option key={col} value={col}>{col}</option>
                      ))}
                    </select>
                  </div>
                </>
              )}

              {dbEngine === 'postgresql' && (
                <div className="p-3 bg-surface-inset border border-edge rounded-lg">
                  <p className="text-sm text-content-muted">Encoding: <span className="text-content font-medium">{editDb.charset}</span></p>
                  <p className="text-sm text-content-muted">Collation: <span className="text-content font-medium">{editDb.collation}</span></p>
                  <p className="text-xs text-content-muted mt-1">PostgreSQL encoding cannot be changed after creation.</p>
                </div>
              )}

              {/* Authorized Users - Multiselect */}
              <div>
                <label className="block text-sm text-content-muted mb-2">Authorized Users</label>
                <div className="max-h-40 overflow-y-auto space-y-1 bg-surface-inset border border-edge rounded-lg p-2">
                  {users.length === 0 ? (
                    <p className="text-xs text-content-muted p-2">No users found</p>
                  ) : (
                    users.map(u => {
                      const key = `${u.user}@${u.host}`;
                      const isChecked = editDb.authorizedUsers.includes(key);
                      const isRoot = u.user.toLowerCase() === 'root';
                      return (
                        <label
                          key={key}
                          className={`flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer hover:bg-hover transition-colors ${
                            isChecked ? 'bg-emerald-500/10' : ''
                          } ${isRoot ? 'opacity-60 cursor-not-allowed' : ''}`}
                        >
                          <input
                            type="checkbox"
                            checked={isChecked}
                            disabled={isRoot}
                            onChange={() => {
                              if (isRoot) return;
                              const updated = isChecked
                                ? editDb.authorizedUsers.filter(k => k !== key)
                                : [...editDb.authorizedUsers, key];
                              setEditDb({ ...editDb, authorizedUsers: updated });
                            }}
                            className="accent-emerald-500 w-4 h-4"
                          />
                          <span className="text-sm text-content">{u.user}</span>
                          <span className="text-xs text-content-muted">@{u.host}</span>
                        </label>
                      );
                    })
                  )}
                </div>
              </div>

              <div className="flex justify-end gap-2">
                <button
                  onClick={() => { setDialog({ type: null }); setEditDb(null); }}
                  className="px-4 py-2 bg-surface-raised hover:bg-hover rounded-lg text-sm transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSaveEditDb}
                  disabled={dialogLoading}
                  className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
                >
                  {dialogLoading ? (
                    <>
                      <RefreshCw className="w-4 h-4 animate-spin" />
                      Saving...
                    </>
                  ) : (
                    'Save'
                  )}
                </button>
              </div>
            </div>
          )}
        </Dialog>
      )}
    </div>
  );
}

// Dialog Component
function Dialog({ title, children, onClose }: { title: string; children: React.ReactNode; onClose: () => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div className="bg-surface border border-edge rounded-xl p-6 w-full max-w-md" onClick={e => e.stopPropagation()}>
        <h3 className="text-lg font-semibold text-content mb-4">{title}</h3>
        {children}
      </div>
    </div>
  );
}
