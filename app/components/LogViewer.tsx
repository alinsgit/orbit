import { useState, useEffect, useRef, useCallback } from 'react';
import {
  FileText, RefreshCw, Trash2, ChevronDown,
  AlertCircle, AlertTriangle, Info, Filter, Search,
  ChevronLeft, ChevronRight, Eye, XCircle, Layers,
  Copy, Check
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

// Service badge config
const SERVICE_BADGE: Record<string, { label: string; color: string; iconColor: string }> = {
  error:            { label: 'nginx-error', color: 'bg-orange-500/20 text-orange-400', iconColor: 'text-orange-400' },
  access:           { label: 'access',      color: 'bg-blue-500/20 text-blue-400',     iconColor: 'text-blue-400' },
  php:              { label: 'php',         color: 'bg-purple-500/20 text-purple-400', iconColor: 'text-purple-400' },
  mariadb:          { label: 'mariadb',     color: 'bg-sky-500/20 text-sky-400',       iconColor: 'text-sky-400' },
  mailpit:          { label: 'mailpit',     color: 'bg-pink-500/20 text-pink-400',     iconColor: 'text-pink-400' },
  meilisearch:      { label: 'meilisearch', color: 'bg-violet-500/20 text-violet-400', iconColor: 'text-violet-400' },
  redis:            { label: 'redis',       color: 'bg-red-500/20 text-red-400',       iconColor: 'text-red-400' },
  'apache-error':   { label: 'apache-error',  color: 'bg-orange-500/20 text-orange-400', iconColor: 'text-orange-400' },
  'apache-access':  { label: 'apache-access', color: 'bg-blue-500/20 text-blue-400',     iconColor: 'text-blue-400' },
  apache:           { label: 'apache',      color: 'bg-orange-500/20 text-orange-400', iconColor: 'text-orange-400' },
  postgresql:       { label: 'postgresql',  color: 'bg-indigo-500/20 text-indigo-400', iconColor: 'text-indigo-400' },
  mongodb:          { label: 'mongodb',     color: 'bg-green-500/20 text-green-400',   iconColor: 'text-green-400' },
  other:            { label: 'other',       color: 'bg-surface-inset text-content-secondary', iconColor: 'text-content-secondary' },
};

// Map log_type to service group (for filtering)
const getServiceGroup = (logType: string): string => {
  if (logType === 'error' || logType === 'access') return 'nginx';
  if (logType === 'apache-error' || logType === 'apache-access') return 'apache';
  return logType;
};

// Service filter labels
const SERVICE_LABELS: Record<string, string> = {
  nginx: 'Nginx',
  apache: 'Apache',
  php: 'PHP',
  mariadb: 'MariaDB',
  postgresql: 'PostgreSQL',
  mongodb: 'MongoDB',
  mailpit: 'Mailpit',
  redis: 'Redis',
  other: 'Other',
};

export function LogViewer() {
  const { addToast } = useApp();
  const [logFiles, setLogFiles] = useState<LogFile[]>([]);
  const [selectedLog, setSelectedLog] = useState<LogFile | null>(null);
  const [logEntries, setLogEntries] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  // Filtering (sent to backend for server-side filtering)
  const [levelFilter, setLevelFilter] = useState<string>('all');
  const [serviceFilter, setServiceFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchInput, setSearchInput] = useState('');

  // Pagination
  const [linesPerPage] = useState(100);
  const [offset, setOffset] = useState(0);
  const [totalLines, setTotalLines] = useState(0);
  const [filteredLines, setFilteredLines] = useState(0);

  // Auto-refresh
  const [autoRefresh, setAutoRefresh] = useState(false);
  const autoRefreshRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Log detail modal
  const [selectedEntry, setSelectedEntry] = useState<LogEntry | null>(null);
  const [copied, setCopied] = useState(false);

  // Search debounce timer
  const searchTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounce search input
  const handleSearchChange = (value: string) => {
    setSearchInput(value);
    if (searchTimerRef.current) clearTimeout(searchTimerRef.current);
    searchTimerRef.current = setTimeout(() => {
      setSearchQuery(value);
      setOffset(0);
    }, 300);
  };

  // Load log files
  const loadLogFiles = useCallback(async () => {
    try {
      const files = await getLogFiles();
      setLogFiles(files);
      setSelectedLog(prev => {
        if (!prev && files.length > 0) {
          return files.find(f => f.log_type === 'error') || files[0];
        }
        return prev;
      });
    } catch (e) {
      console.error('Failed to load log files:', e);
      addToast({ type: 'error', message: 'Failed to load log files' });
    }
  }, [addToast]);

  // Load log entries with server-side filtering
  const loadLogEntries = useCallback(async (refresh = false) => {
    if (!selectedLog) return;

    if (refresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }

    try {
      const result = await readLogFile(
        selectedLog.path,
        linesPerPage,
        offset,
        levelFilter,
        searchQuery,
      );
      setLogEntries(result.entries);
      setTotalLines(result.total_lines);
      setFilteredLines(result.filtered_lines);
    } catch (e) {
      console.error('Failed to load log entries:', e);
      addToast({ type: 'error', message: 'Failed to load log entries' });
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [selectedLog, linesPerPage, offset, levelFilter, searchQuery, addToast]);

  // Initial load
  useEffect(() => {
    loadLogFiles();
  }, [loadLogFiles]);

  // Load entries when log, offset, or filters change
  useEffect(() => {
    if (selectedLog) {
      loadLogEntries();
    }
  }, [loadLogEntries, selectedLog]);

  // Reset offset when selected log or level filter changes
  const handleSelectLog = (file: LogFile) => {
    setSelectedLog(file);
    setOffset(0);
  };

  const handleLevelFilterChange = (value: string) => {
    setLevelFilter(value);
    setOffset(0);
  };

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
        autoRefreshRef.current = null;
      }
    };
  }, [autoRefresh, loadLogEntries, selectedLog]);

  // Toggle live mode — reset to latest entries
  const handleToggleLive = () => {
    const newValue = !autoRefresh;
    setAutoRefresh(newValue);
    if (newValue) {
      setOffset(0);
    }
  };

  // Clear selected log
  const handleClearLog = async () => {
    if (!selectedLog) return;
    try {
      await clearLogFile(selectedLog.path);
      addToast({ type: 'success', message: `${selectedLog.name} cleared` });
      loadLogEntries();
      loadLogFiles();
    } catch (e) {
      console.error('Failed to clear log:', e);
      addToast({ type: 'error', message: `Failed to clear log: ${e}` });
    }
  };

  // Clear all logs
  const handleClearAllLogs = async () => {
    try {
      const cleared = await clearAllLogs();
      addToast({ type: 'success', message: `${cleared} log files cleared` });
      loadLogFiles();
      if (selectedLog) {
        loadLogEntries();
      }
    } catch (e) {
      console.error('Failed to clear all logs:', e);
      addToast({ type: 'error', message: `Failed to clear logs: ${e}` });
    }
  };

  // Copy log entry to clipboard
  const handleCopyEntry = async (raw: string) => {
    try {
      await navigator.clipboard.writeText(raw);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      addToast({ type: 'error', message: 'Failed to copy to clipboard' });
    }
  };

  // Derive available service groups from actual log files
  const availableServices = [...new Set(logFiles.map(f => getServiceGroup(f.log_type)))].sort();

  // Filter sidebar files by service
  const filteredLogFiles = serviceFilter === 'all'
    ? logFiles
    : logFiles.filter(f => getServiceGroup(f.log_type) === serviceFilter);

  // Pagination info
  const displayTotal = filteredLines;
  const totalPages = Math.max(1, Math.ceil(displayTotal / linesPerPage));
  const currentPage = Math.floor(offset / linesPerPage) + 1;

  const getBadge = (logType: string) => SERVICE_BADGE[logType] || SERVICE_BADGE.other;

  return (
    <div className="p-6 h-full flex flex-col">
      {/* Header */}
      <header className="mb-6">
        <h2 className="text-2xl font-bold">Logs</h2>
        <p className="text-content-secondary">View and analyze server logs</p>
      </header>

      <div className="flex gap-6 flex-1 min-h-0">
        {/* Log Files Sidebar */}
        <aside className="w-64 flex-shrink-0 flex flex-col">
          {/* Sidebar header with service filter */}
          <div className="flex items-center justify-between mb-3">
            <h3 className="font-semibold text-sm text-content-secondary">Log Files</h3>
            {availableServices.length > 1 && (
              <div className="relative">
                <Layers size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
                <select
                  value={serviceFilter}
                  onChange={(e) => setServiceFilter(e.target.value)}
                  className="pl-6 pr-5 py-1 bg-surface-raised border border-edge rounded-lg text-xs focus:outline-none focus:border-emerald-500 appearance-none cursor-pointer"
                >
                  <option value="all">All</option>
                  {availableServices.map(s => (
                    <option key={s} value={s}>{SERVICE_LABELS[s] || s}</option>
                  ))}
                </select>
                <ChevronDown size={10} className="absolute right-1.5 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
              </div>
            )}
          </div>

          {/* File list */}
          <div className="flex-1 min-h-0 overflow-y-auto space-y-2">
            {filteredLogFiles.length === 0 ? (
              <p className="text-sm text-content-muted">No log files found</p>
            ) : (
              filteredLogFiles.map((file) => {
                const badge = getBadge(file.log_type);
                const isSelected = selectedLog?.path === file.path;
                return (
                  <div
                    key={file.path}
                    role="button"
                    tabIndex={0}
                    onClick={() => handleSelectLog(file)}
                    onKeyDown={(e) => { if (e.key === 'Enter') handleSelectLog(file); }}
                    className={`w-full text-left p-3 rounded-lg transition-all cursor-pointer ${
                      isSelected
                        ? 'bg-emerald-600/20 border border-emerald-500/50'
                        : 'bg-surface-raised border border-edge-subtle hover:bg-surface-raised'
                    }`}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <FileText size={14} className={badge.iconColor} />
                      <span className="text-sm font-medium truncate flex-1">{file.name}</span>
                      {isSelected && (
                        <button
                          onClick={(e) => { e.stopPropagation(); handleClearLog(); }}
                          className="p-0.5 hover:bg-red-500/20 rounded text-content-muted hover:text-red-400 transition-colors"
                          title="Clear this log"
                        >
                          <Trash2 size={12} />
                        </button>
                      )}
                    </div>
                    <div className="flex items-center justify-between text-xs text-content-muted">
                      <span>{formatBytes(file.size)}</span>
                      <span className={`px-1.5 py-0.5 rounded text-xs ${badge.color}`}>
                        {badge.label}
                      </span>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </aside>

        {/* Log Content */}
        <main className="flex-1 flex flex-col min-w-0">
          {selectedLog ? (
            <>
              {/* Toolbar — search + level filter + actions */}
              <div className="flex items-center gap-3 mb-4">
                {/* Search */}
                <div className="relative flex-1 max-w-md">
                  <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-content-muted" />
                  <input
                    type="text"
                    value={searchInput}
                    onChange={(e) => handleSearchChange(e.target.value)}
                    placeholder="Search logs..."
                    className="w-full pl-10 pr-4 py-2 bg-surface-raised border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500"
                  />
                </div>

                {/* Level Filter */}
                <div className="relative">
                  <Filter size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-content-muted" />
                  <select
                    value={levelFilter}
                    onChange={(e) => handleLevelFilterChange(e.target.value)}
                    className="pl-9 pr-8 py-2 bg-surface-raised border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500 appearance-none cursor-pointer"
                  >
                    <option value="all">All Levels</option>
                    <option value="error">Errors</option>
                    <option value="warning">Warnings</option>
                    <option value="info">Info</option>
                  </select>
                  <ChevronDown size={14} className="absolute right-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
                </div>

                {/* Separator */}
                <div className="w-px h-6 bg-edge" />

                {/* Live */}
                <button
                  onClick={handleToggleLive}
                  className={`flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer ${
                    autoRefresh
                      ? 'bg-emerald-600 text-white'
                      : 'bg-surface-raised hover:bg-hover text-content-secondary'
                  }`}
                >
                  <Eye size={14} className={autoRefresh ? 'animate-pulse' : ''} />
                  Live
                </button>

                {/* Refresh */}
                <button
                  onClick={() => loadLogEntries(true)}
                  disabled={refreshing}
                  className="p-2 bg-surface-raised hover:bg-hover rounded-lg transition-colors cursor-pointer disabled:opacity-50"
                  title="Refresh"
                >
                  <RefreshCw size={14} className={refreshing ? 'animate-spin' : ''} />
                </button>

                {/* Clear All */}
                <button
                  onClick={handleClearAllLogs}
                  disabled={logFiles.length === 0}
                  className="flex items-center gap-1.5 px-3 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg text-sm font-medium transition-colors cursor-pointer disabled:opacity-50"
                  title="Clear All Logs"
                >
                  <Trash2 size={14} />
                  Clear All
                </button>
              </div>

              {/* Log Entries */}
              <div className="flex-1 bg-surface-inset border border-edge rounded-xl overflow-hidden flex flex-col">
                {loading ? (
                  <div className="flex-1 flex items-center justify-center">
                    <RefreshCw size={24} className="animate-spin text-content-muted" />
                  </div>
                ) : logEntries.length === 0 ? (
                  <div className="flex-1 flex items-center justify-center text-content-muted">
                    No log entries found
                  </div>
                ) : (
                  <div className="flex-1 overflow-y-auto font-mono text-xs">
                    {logEntries.map((entry, index) => {
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
                    {filteredLines === totalLines
                      ? `${totalLines.toLocaleString()} entries`
                      : `${filteredLines.toLocaleString()} of ${totalLines.toLocaleString()} entries`
                    }
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
                      Page {currentPage} of {totalPages}
                    </span>
                    <button
                      onClick={() => setOffset(offset + linesPerPage)}
                      disabled={currentPage >= totalPages}
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
          onClick={() => { setSelectedEntry(null); setCopied(false); }}
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
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleCopyEntry(selectedEntry.raw)}
                  className="flex items-center gap-1.5 px-3 py-1.5 bg-surface-raised hover:bg-hover rounded-lg text-xs text-content-secondary hover:text-content transition-colors cursor-pointer"
                  title="Copy to clipboard"
                >
                  {copied ? <Check size={14} className="text-emerald-500" /> : <Copy size={14} />}
                  {copied ? 'Copied' : 'Copy'}
                </button>
                <button
                  onClick={() => { setSelectedEntry(null); setCopied(false); }}
                  className="p-1 hover:bg-surface-raised rounded cursor-pointer"
                >
                  <XCircle size={20} />
                </button>
              </div>
            </div>
            <div className="flex-1 overflow-auto p-4">
              <pre
                className="font-mono text-sm text-content-secondary whitespace-pre-wrap break-all"
                style={{ userSelect: 'text' }}
              >
                {selectedEntry.raw}
              </pre>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
