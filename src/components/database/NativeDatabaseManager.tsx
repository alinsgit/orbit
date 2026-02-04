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
} from 'lucide-react';
import {
  dbConnect,
  dbDisconnect,
  isConnected,
  getServerInfo,
  listDatabases,
  createDatabase,
  dropDatabase,
  listUsers,
  createUser,
  dropUser,
  changePassword,
  grantPrivileges,
  type DatabaseInfo,
  type UserInfo,
  type ServerInfo,
  type DbConnectionConfig,
} from '../../lib/db-api';

type Tab = 'databases' | 'users';

interface DialogState {
  type: 'createDb' | 'createUser' | 'changePassword' | 'grantPrivileges' | null;
  data?: UserInfo;
}

export default function NativeDatabaseManager() {
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>('databases');

  // Connection form
  const [config, setConfig] = useState<DbConnectionConfig>({
    host: '127.0.0.1',
    port: 3306,
    user: 'root',
    password: '',
  });
  const [showPassword, setShowPassword] = useState(false);

  // Data
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const [databases, setDatabases] = useState<DatabaseInfo[]>([]);
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [loading, setLoading] = useState(false);

  // Dialogs
  const [dialog, setDialog] = useState<DialogState>({ type: null });
  const [dialogLoading, setDialogLoading] = useState(false);

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
    setConnected(alreadyConnected);
    if (alreadyConnected) {
      loadData();
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
      console.log('Setting serverInfo:', info);
      setServerInfo(info);
    } catch (err) {
      console.error('getServerInfo error:', err);
    }

    try {
      const dbs = await listDatabases();
      console.log('Setting databases:', dbs);
      setDatabases(dbs);
    } catch (err) {
      console.error('listDatabases error:', err);
    }

    try {
      const userList = await listUsers();
      console.log('Setting users:', userList);
      setUsers(userList);
    } catch (err) {
      console.error('listUsers error:', err);
    }

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
    try {
      setLoading(true);
      await dropDatabase(name);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to drop database');
    } finally {
      setLoading(false);
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
    try {
      setLoading(true);
      await dropUser(user.user, user.host);
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to drop user');
    } finally {
      setLoading(false);
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

  // Not connected - show login form
  if (!connected) {
    return (
      <div className="p-6 h-full overflow-y-auto">
        <div className="max-w-md mx-auto mt-12">
          <div className="flex items-center gap-3 mb-6">
            <Database className="w-8 h-8 text-emerald-500" />
            <div>
              <h2 className="text-xl font-semibold text-content">Database Manager</h2>
              <p className="text-sm text-content-muted">Connect to MariaDB</p>
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
            <button
              onClick={() => setDialog({ type: 'createDb' })}
              className="flex items-center gap-2 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm transition-colors"
            >
              <Plus className="w-4 h-4" />
              Create Database
            </button>
          </div>

          <div className="space-y-2">
            {databases.map(db => (
              <div
                key={db.name}
                className="flex items-center justify-between p-3 bg-surface-raised rounded-lg hover:bg-hover transition-colors"
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
                {!['mysql', 'information_schema', 'performance_schema', 'sys'].includes(db.name.toLowerCase()) && (
                  <button
                    onClick={() => handleDropDatabase(db.name)}
                    className="p-2 text-red-400 hover:bg-red-500/10 rounded-lg transition-colors"
                    title="Delete database"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                )}
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
                  {user.user.toLowerCase() !== 'root' && (
                    <button
                      onClick={() => handleDropUser(user)}
                      className="p-2 text-red-400 hover:bg-red-500/10 rounded-lg transition-colors"
                      title="Delete user"
                    >
                      <Trash2 className="w-4 h-4" />
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
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors"
              >
                {dialogLoading ? 'Creating...' : 'Create'}
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
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors"
              >
                {dialogLoading ? 'Creating...' : 'Create'}
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
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors"
              >
                {dialogLoading ? 'Changing...' : 'Change Password'}
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
                  .filter(db => !['mysql', 'information_schema', 'performance_schema', 'sys'].includes(db.name.toLowerCase()))
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
                className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 rounded-lg text-sm transition-colors"
              >
                {dialogLoading ? 'Granting...' : 'Grant Privileges'}
              </button>
            </div>
          </div>
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
