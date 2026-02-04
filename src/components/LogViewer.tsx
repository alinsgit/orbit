import { useState, useEffect, useRef } from 'react';
import {
  FileText, RefreshCw, Trash2, ChevronDown,
  AlertCircle, AlertTriangle, Info, Filter, Search,
  ChevronLeft, ChevronRight, Eye, XCircle
} from 'lucide-react';
import { getLogFiles, readLogFile, clearLogFile, clearAllLogs, LogFile, LogEntry } from '../lib/api';
import { useApp } from '../lib/AppContext';

// Format bytes to human readable
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

// Log level colors and icons
const LOG_LEVEL_CONFIG: Record<string, { color: string; bgColor: string; icon: React.ReactNode }> = {
  error: {
    color: 'text-red-400',
    bgColor: 'bg-red-500/10 border-red-500/30',
    icon: <AlertCircle size={14} />
  },
  warning: {
    color: 'text-amber-400',
    bgColor: 'bg-amber-500/10 border-amber-500/30',
    icon: <AlertTriangle size={14} />
  },
  info: {
    color: 'text-blue-400',
    bgColor: 'bg-surface-raised border-edge-subtle',
    icon: <Info size={14} />
  }
};

export function LogViewer() {
  const { addToast } = useApp();
  const [logFiles, setLogFiles] = useState<LogFile[]>([]);
  const [selectedLog, setSelectedLog] = useState<LogFile | null>(null);
  const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  // Filtering
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Pagination
  const [linesPerPage] = useState(100);
  const [offset, setOffset] = useState(0);

  // Auto-refresh
  const [autoRefresh, setAutoRefresh] = useState(false);
  const autoRefreshRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Log detail modal
  const [selectedEntry, setSelectedEntry] = useState<LogEntry | null>(null);

  // Load log files
  const loadLogFiles = async () => {
    try {
      const files = await getLogFiles();
      setLogFiles(files);
      // Auto-select first error log if available
      if (!selectedLog && files.length > 0) {
        const errorLog = files.find(f => f.log_type === 'error') || files[0];
        setSelectedLog(errorLog);
      }
    } catch (e) {
      console.error('Failed to load log files:', e);
    }
  };

  // Load log entries
  const loadLogEntries = async (refresh = false) => {
    if (!selectedLog) return;

    if (refresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }

    try {
      const entries = await readLogFile(selectedLog.path, linesPerPage, offset);
      setLogEntries(entries);
    } catch (e) {
      console.error('Failed to load log entries:', e);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  // Initial load
  useEffect(() => {
    loadLogFiles();
  }, []);

  // Load entries when log changes
  useEffect(() => {
    if (selectedLog) {
      setOffset(0);
      loadLogEntries();
    }
  }, [selectedLog]);

  // Load entries when offset changes
  useEffect(() => {
    if (selectedLog) {
      loadLogEntries();
    }
  }, [offset]);

  // Auto-refresh
  useEffect(() => {
    if (autoRefresh && selectedLog) {
      autoRefreshRef.current = setInterval(() => {
        loadLogEntries(true);
      }, 3000);
    }
    return () => {
      if (autoRefreshRef.current) {
        clearInterval(autoRefreshRef.current);
      }
    };
  }, [autoRefresh, selectedLog]);

  // Clear log
  const handleClearLog = async () => {
    if (!selectedLog) return;
    try {
      await clearLogFile(selectedLog.path);
      loadLogEntries();
      loadLogFiles();
    } catch (e) {
      console.error('Failed to clear log:', e);
    }
  };

  // Clear all logs
  const handleClearAllLogs = async () => {
    try {
      await clearAllLogs();
      loadLogFiles();
      if (selectedLog) {
        loadLogEntries();
      }
    } catch (e) {
      console.error('Failed to clear all logs:', e);
      addToast({ type: 'error', message: 'Failed to clear logs: ' + e });
    }
  };

  // Filter entries
  const filteredEntries = logEntries.filter(entry => {
    if (levelFilter !== 'all' && entry.level !== levelFilter) return false;
    if (searchQuery && !entry.raw.toLowerCase().includes(searchQuery.toLowerCase())) return false;
    return true;
  });

  return (
    <div className="p-6 h-full flex flex-col">
      {/* Header */}
      <header className="flex justify-between items-center mb-6">
        <div>
          <h2 className="text-2xl font-bold">Logs</h2>
          <p className="text-content-secondary">View and analyze server logs</p>
        </div>
        <div className="flex items-center gap-3">
          {/* Auto Refresh Toggle */}
          <button
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer ${
              autoRefresh
                ? 'bg-emerald-600 text-white'
                : 'bg-surface-raised hover:bg-hover text-content-secondary'
            }`}
          >
            <Eye size={16} className={autoRefresh ? 'animate-pulse' : ''} />
            Live
          </button>

          {/* Refresh */}
          <button
            onClick={() => loadLogEntries(true)}
            disabled={refreshing || !selectedLog}
            className="p-2 bg-surface-raised hover:bg-hover rounded-lg transition-colors cursor-pointer disabled:opacity-50"
            title="Refresh"
          >
            <RefreshCw size={16} className={refreshing ? 'animate-spin' : ''} />
          </button>

          {/* Clear All Logs */}
          <button
            onClick={handleClearAllLogs}
            disabled={logFiles.length === 0}
            className="flex items-center gap-2 px-3 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg text-sm font-medium transition-colors cursor-pointer disabled:opacity-50"
            title="Clear All Logs"
          >
            <Trash2 size={16} />
            Clear All
          </button>
        </div>
      </header>

      <div className="flex gap-6 flex-1 min-h-0">
        {/* Log Files Sidebar */}
        <aside className="w-64 flex-shrink-0 flex flex-col">
          <h3 className="font-semibold text-sm text-content-secondary mb-3">Log Files</h3>
          <div className="flex-1 min-h-0 overflow-y-auto space-y-2">
            {logFiles.length === 0 ? (
              <p className="text-sm text-content-muted">No log files found</p>
            ) : (
              logFiles.map((file) => (
                <button
                  key={file.path}
                  onClick={() => setSelectedLog(file)}
                  className={`w-full text-left p-3 rounded-lg transition-all cursor-pointer ${
                    selectedLog?.path === file.path
                      ? 'bg-emerald-600/20 border border-emerald-500/50'
                      : 'bg-surface-raised border border-edge-subtle hover:bg-surface-raised'
                  }`}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <FileText size={14} className={
                      file.log_type === 'error' ? 'text-red-400' :
                      file.log_type === 'access' ? 'text-blue-400' :
                      file.log_type === 'php' ? 'text-purple-400' :
                      'text-content-secondary'
                    } />
                    <span className="text-sm font-medium truncate">{file.name}</span>
                  </div>
                  <div className="flex items-center justify-between text-xs text-content-muted">
                    <span>{formatBytes(file.size)}</span>
                    <span className={`px-1.5 py-0.5 rounded text-xs ${
                      file.log_type === 'error' ? 'bg-red-500/20 text-red-400' :
                      file.log_type === 'access' ? 'bg-blue-500/20 text-blue-400' :
                      file.log_type === 'php' ? 'bg-purple-500/20 text-purple-400' :
                      'bg-surface-inset text-content-secondary'
                    }`}>{file.log_type}</span>
                  </div>
                </button>
              ))
            )}
          </div>
        </aside>

        {/* Log Content */}
        <main className="flex-1 flex flex-col min-w-0">
          {selectedLog ? (
            <>
              {/* Toolbar */}
              <div className="flex items-center gap-3 mb-4">
                {/* Search */}
                <div className="relative flex-1 max-w-md">
                  <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-content-muted" />
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder="Search logs..."
                    className="w-full pl-10 pr-4 py-2 bg-surface-raised border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500"
                  />
                </div>

                {/* Level Filter */}
                <div className="relative">
                  <Filter size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-content-muted" />
                  <select
                    value={levelFilter}
                    onChange={(e) => setLevelFilter(e.target.value)}
                    className="pl-9 pr-8 py-2 bg-surface-raised border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500 appearance-none cursor-pointer"
                  >
                    <option value="all">All Levels</option>
                    <option value="error">Errors</option>
                    <option value="warning">Warnings</option>
                    <option value="info">Info</option>
                  </select>
                  <ChevronDown size={14} className="absolute right-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
                </div>

                {/* Clear Log */}
                <button
                  onClick={handleClearLog}
                  className="flex items-center gap-2 px-3 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg text-sm transition-colors cursor-pointer"
                >
                  <Trash2 size={14} />
                  Clear
                </button>
              </div>

              {/* Log Entries */}
              <div className="flex-1 bg-surface-inset border border-edge rounded-xl overflow-hidden flex flex-col">
                {loading ? (
                  <div className="flex-1 flex items-center justify-center">
                    <RefreshCw size={24} className="animate-spin text-content-muted" />
                  </div>
                ) : filteredEntries.length === 0 ? (
                  <div className="flex-1 flex items-center justify-center text-content-muted">
                    No log entries found
                  </div>
                ) : (
                  <div className="flex-1 overflow-y-auto font-mono text-xs">
                    {filteredEntries.map((entry, index) => {
                      const config = LOG_LEVEL_CONFIG[entry.level] || LOG_LEVEL_CONFIG.info;
                      return (
                        <div
                          key={index}
                          onClick={() => setSelectedEntry(entry)}
                          className={`px-4 py-2 border-b border-edge/50 hover:bg-surface-raised/30 cursor-pointer ${config.bgColor}`}
                        >
                          <div className="flex items-start gap-3">
                            <span className={config.color}>{config.icon}</span>
                            {entry.timestamp && (
                              <span className="text-content-muted flex-shrink-0 w-40">
                                {entry.timestamp}
                              </span>
                            )}
                            <span className="text-content-secondary break-all">
                              {entry.message.length > 200
                                ? entry.message.substring(0, 200) + '...'
                                : entry.message}
                            </span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* Pagination */}
                <div className="flex items-center justify-between px-4 py-3 border-t border-edge bg-surface/80">
                  <span className="text-xs text-content-muted">
                    Showing {filteredEntries.length} of {logEntries.length} entries
                  </span>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setOffset(Math.max(0, offset - linesPerPage))}
                      disabled={offset === 0}
                      className="p-1.5 bg-surface-raised hover:bg-hover rounded disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
                    >
                      <ChevronLeft size={14} />
                    </button>
                    <span className="text-xs text-content-secondary px-2">
                      Page {Math.floor(offset / linesPerPage) + 1}
                    </span>
                    <button
                      onClick={() => setOffset(offset + linesPerPage)}
                      disabled={logEntries.length < linesPerPage}
                      className="p-1.5 bg-surface-raised hover:bg-hover rounded disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
                    >
                      <ChevronRight size={14} />
                    </button>
                  </div>
                </div>
              </div>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center text-content-muted">
              <div className="text-center">
                <FileText size={48} className="mx-auto mb-4 opacity-50" />
                <p>Select a log file to view</p>
              </div>
            </div>
          )}
        </main>
      </div>

      {/* Log Entry Detail Modal */}
      {selectedEntry && (
        <div
          className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-8"
          onClick={() => setSelectedEntry(null)}
        >
          <div
            className="bg-surface border border-edge rounded-xl max-w-4xl w-full max-h-[80vh] flex flex-col"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between p-4 border-b border-edge">
              <h3 className="font-semibold flex items-center gap-2">
                {LOG_LEVEL_CONFIG[selectedEntry.level]?.icon}
                <span className={LOG_LEVEL_CONFIG[selectedEntry.level]?.color}>
                  {selectedEntry.level.toUpperCase()}
                </span>
                {selectedEntry.timestamp && (
                  <span className="text-content-muted text-sm ml-2">
                    {selectedEntry.timestamp}
                  </span>
                )}
              </h3>
              <button
                onClick={() => setSelectedEntry(null)}
                className="p-1 hover:bg-surface-raised rounded cursor-pointer"
              >
                <XCircle size={20} />
              </button>
            </div>
            <div className="flex-1 overflow-auto p-4">
              <pre className="font-mono text-sm text-content-secondary whitespace-pre-wrap break-all">
                {selectedEntry.raw}
              </pre>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
