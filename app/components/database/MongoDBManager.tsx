import { useState, useEffect, useCallback } from 'react';
import {
  Database,
  RefreshCw,
  Trash2,
  AlertCircle,
  ChevronRight,
  Server,
  HardDrive,
  ArrowLeft,
} from 'lucide-react';
import { ask } from '@tauri-apps/plugin-dialog';
import {
  mongoListDatabases,
  mongoListCollections,
  mongoDbStats,
  mongoDropDatabase,
} from '../../lib/api';

interface DbStats {
  db: string;
  collections: number;
  dataSize: number;
  storageSize: number;
  objects: number;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export default function MongoDBManager() {
  const [databases, setDatabases] = useState<string[]>([]);
  const [collections, setCollections] = useState<string[]>([]);
  const [selectedDb, setSelectedDb] = useState<string | null>(null);
  const [dbStats, setDbStats] = useState<DbStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const loadDatabases = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const dbs = await mongoListDatabases();
      setDatabases(dbs);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDatabases();
  }, [loadDatabases]);

  // Auto-dismiss success messages
  useEffect(() => {
    if (success) {
      const t = setTimeout(() => setSuccess(null), 3000);
      return () => clearTimeout(t);
    }
  }, [success]);

  const handleSelectDb = async (db: string) => {
    setSelectedDb(db);
    setError(null);
    try {
      const [cols, statsRaw] = await Promise.all([
        mongoListCollections(db),
        mongoDbStats(db),
      ]);
      setCollections(cols);
      try {
        const parsed = JSON.parse(statsRaw);
        setDbStats({
          db: parsed.db || db,
          collections: parsed.collections || 0,
          dataSize: parsed.dataSize || 0,
          storageSize: parsed.storageSize || 0,
          objects: parsed.objects || 0,
        });
      } catch {
        setDbStats(null);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDropDatabase = async (db: string) => {
    const confirmed = await ask(`Are you sure you want to drop database "${db}"? This action cannot be undone.`, {
      title: 'Drop Database',
      kind: 'warning',
    });
    if (!confirmed) return;

    try {
      await mongoDropDatabase(db);
      setSuccess(`Database '${db}' dropped successfully`);
      setSelectedDb(null);
      setCollections([]);
      setDbStats(null);
      await loadDatabases();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const systemDbs = ['admin', 'local', 'config'];

  return (
    <div className="h-full overflow-y-auto p-4 space-y-4">
      {/* Messages */}
      {error && (
        <div className="flex items-center gap-2 p-3 bg-red-500/10 text-red-400 rounded-lg text-sm">
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          {error}
        </div>
      )}
      {success && (
        <div className="flex items-center gap-2 p-3 bg-emerald-500/10 text-emerald-400 rounded-lg text-sm">
          {success}
        </div>
      )}

      {/* Connection Info */}
      <div className="bg-surface-raised rounded-xl p-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <Server className="w-4 h-4 text-emerald-500" />
            <h3 className="text-sm font-medium text-content-secondary">MongoDB Connection</h3>
          </div>
          <button
            onClick={loadDatabases}
            disabled={loading}
            className="p-1.5 text-content-muted hover:text-content hover:bg-surface rounded-md transition-colors"
            title="Refresh"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>
        <div className="grid grid-cols-3 gap-4 text-sm">
          <div>
            <span className="text-content-muted">Host:</span>
            <code className="ml-2 text-content-secondary bg-surface px-2 py-0.5 rounded">127.0.0.1:27017</code>
          </div>
          <div>
            <span className="text-content-muted">Auth:</span>
            <code className="ml-2 text-content-secondary bg-surface px-2 py-0.5 rounded">Disabled</code>
          </div>
          <div>
            <span className="text-content-muted">Databases:</span>
            <code className="ml-2 text-content-secondary bg-surface px-2 py-0.5 rounded">{databases.length}</code>
          </div>
        </div>
      </div>

      {/* Detail view */}
      {selectedDb ? (
        <div className="bg-surface-raised rounded-xl p-4 space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <button
                onClick={() => { setSelectedDb(null); setCollections([]); setDbStats(null); }}
                className="p-1 text-content-muted hover:text-content hover:bg-surface rounded-md transition-colors"
              >
                <ArrowLeft className="w-4 h-4" />
              </button>
              <HardDrive className="w-4 h-4 text-emerald-500" />
              <h3 className="text-sm font-medium text-content">{selectedDb}</h3>
              {systemDbs.includes(selectedDb) && (
                <span className="text-xs px-2 py-0.5 bg-amber-500/20 text-amber-400 rounded">system</span>
              )}
            </div>
            {!systemDbs.includes(selectedDb) && (
              <button
                onClick={() => handleDropDatabase(selectedDb)}
                className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
                Drop
              </button>
            )}
          </div>

          {/* Stats */}
          {dbStats && (
            <div className="grid grid-cols-4 gap-3">
              {[
                { label: 'Collections', value: dbStats.collections },
                { label: 'Objects', value: dbStats.objects.toLocaleString() },
                { label: 'Data Size', value: formatBytes(dbStats.dataSize) },
                { label: 'Storage', value: formatBytes(dbStats.storageSize) },
              ].map(s => (
                <div key={s.label} className="bg-surface rounded-lg p-3 text-center">
                  <p className="text-lg font-semibold text-content">{s.value}</p>
                  <p className="text-xs text-content-muted">{s.label}</p>
                </div>
              ))}
            </div>
          )}

          {/* Collections list */}
          <div>
            <h4 className="text-sm font-medium text-content-secondary mb-2">Collections ({collections.length})</h4>
            {collections.length === 0 ? (
              <p className="text-sm text-content-muted">No collections in this database.</p>
            ) : (
              <div className="space-y-1">
                {collections.map(col => (
                  <div
                    key={col}
                    className="flex items-center gap-2 px-3 py-2 bg-surface rounded-lg text-sm text-content"
                  >
                    <Database className="w-3.5 h-3.5 text-content-muted" />
                    {col}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      ) : (
        /* Database list */
        <div className="bg-surface-raised rounded-xl p-4">
          <h3 className="text-sm font-medium text-content-secondary mb-3">Databases</h3>
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <RefreshCw className="w-5 h-5 animate-spin text-content-muted" />
            </div>
          ) : databases.length === 0 ? (
            <p className="text-sm text-content-muted py-4 text-center">
              No databases found. Make sure MongoDB is running.
            </p>
          ) : (
            <div className="space-y-1">
              {databases.map(db => (
                <button
                  key={db}
                  onClick={() => handleSelectDb(db)}
                  className="w-full flex items-center justify-between px-3 py-2.5 bg-surface hover:bg-surface-alt rounded-lg transition-colors group"
                >
                  <div className="flex items-center gap-2">
                    <HardDrive className="w-4 h-4 text-content-muted" />
                    <span className="text-sm font-medium text-content">{db}</span>
                    {systemDbs.includes(db) && (
                      <span className="text-xs px-1.5 py-0.5 bg-amber-500/20 text-amber-400 rounded">system</span>
                    )}
                  </div>
                  <ChevronRight className="w-4 h-4 text-content-muted opacity-0 group-hover:opacity-100 transition-opacity" />
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
