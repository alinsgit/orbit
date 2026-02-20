import { useState, useEffect } from 'react';
import { Settings, Loader2, Sun, Moon, Monitor, Palette, Trash2, Eraser, Info, ExternalLink, Github, Heart, ArrowUpCircle, Sparkles, RotateCcw, FolderOpen, RefreshCw, CheckCircle, Terminal, Network, Zap } from 'lucide-react';
import { clearAllCaches } from '../lib/api';
import { useApp } from '../lib/AppContext';
import { PathEditorModal } from './PathEditorModal';
import { HostsEditorModal } from './HostsEditorModal';
import { useTheme, Theme } from '../lib/ThemeContext';
import { getVersion, getTauriVersion } from '@tauri-apps/api/app';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { open as openUrl } from '@tauri-apps/plugin-shell';
import { Store, load } from '@tauri-apps/plugin-store';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

let settingsStore: Store | null = null;
const getSettingsStore = async () => {
  if (!settingsStore) {
    settingsStore = await load('.settings.json', { autoSave: false, defaults: { workspacePath: '' } });
  }
  return settingsStore;
};

export function SettingsManager() {
  const { addToast } = useApp();
  const { theme, setTheme, resolvedTheme } = useTheme();

  // Cache Clear State
  const [clearingCache, setClearingCache] = useState(false);

  // Advanced Tools State
  const [showPathEditor, setShowPathEditor] = useState(false);
  const [showHostsEditor, setShowHostsEditor] = useState(false);

  // About State
  const [appVersion, setAppVersion] = useState<string>('');
  const [tauriVersion, setTauriVersion] = useState<string>('');

  // Update Checker State
  const [updateAvailable, setUpdateAvailable] = useState<Update | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [updateProgress, setUpdateProgress] = useState(0);
  const [updateStatus, setUpdateStatus] = useState<string>('');

  // Workspace Path State
  const [workspacePath, setWorkspacePath] = useState<string>('');
  const [workspaceLoading, setWorkspaceLoading] = useState(false);

  // Ngrok Token State
  const [ngrokToken, setNgrokToken] = useState<string>('');
  const [ngrokTokenSaving, setNgrokTokenSaving] = useState(false);

  useEffect(() => {
    loadVersionInfo();
    handleCheckForUpdates();
    loadWorkspaceSettings();
  }, []);

  const loadWorkspaceSettings = async () => {
    try {
      const store = await getSettingsStore();
      const savedPath = await store.get<string>('workspacePath');
      if (savedPath) setWorkspacePath(savedPath);

      const savedToken = await store.get<string>('ngrokAuthToken');
      if (savedToken) setNgrokToken(savedToken);
    } catch (e) {
      console.error('Failed to load workspace settings:', e);
    }
  };

  const handleChangeWorkspace = async () => {
    try {
      setWorkspaceLoading(true);
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: 'Select Workspace Directory',
      });
      if (selected && typeof selected === 'string') {
        const store = await getSettingsStore();
        setWorkspacePath(selected);
        await store.set('workspacePath', selected);
        await store.save();
        addToast({ type: 'success', message: 'Workspace directory updated!' });
      }
    } catch (e) {
      console.error('Failed to pick workspace dir:', e);
      addToast({ type: 'error', message: `Could not select directory: ${e}` });
    } finally {
      setWorkspaceLoading(false);
    }
  };

  const handleSaveNgrokToken = async () => {
    try {
      setNgrokTokenSaving(true);
      const store = await getSettingsStore();
      await store.set('ngrokAuthToken', ngrokToken.trim());
      await store.save();
      addToast({ type: 'success', message: 'Ngrok Auth Token saved!' });
    } catch (e) {
      console.error('Failed to save Ngrok token:', e);
      addToast({ type: 'error', message: `Could not save token: ${e}` });
    } finally {
      setNgrokTokenSaving(false);
    }
  };

  const loadVersionInfo = async () => {
    try {
      const [version, tauri] = await Promise.all([
        getVersion(),
        getTauriVersion()
      ]);
      setAppVersion(version);
      setTauriVersion(tauri);
    } catch (e) {
      console.error('Failed to load version info:', e);
    }
  };

  const handleCheckForUpdates = async () => {
    setCheckingUpdate(true);
    try {
      const update = await check();
      if (update) {
        setUpdateAvailable(update);
      } else {
        setUpdateAvailable(null);
        addToast({ type: 'success', message: 'You are on the latest version!' });
      }
    } catch (e) {
      console.error('Failed to check for updates:', e);
      addToast({ type: 'error', message: `Update check failed: ${e}` });
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleInstallUpdate = async () => {
    if (!updateAvailable) return;
    setUpdating(true);
    setUpdateProgress(0);
    setUpdateStatus('Downloading...');
    try {
      let totalSize = 0;
      let downloaded = 0;
      await updateAvailable.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            totalSize = event.data.contentLength ?? 0;
            setUpdateStatus('Downloading update...');
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (totalSize > 0) {
              setUpdateProgress(Math.round((downloaded / totalSize) * 100));
            }
            break;
          case 'Finished':
            setUpdateStatus('Installing...');
            setUpdateProgress(100);
            break;
        }
      });
      setUpdateStatus('Restarting...');
      await relaunch();
    } catch (e: any) {
      console.error('Failed to install update:', e);
      addToast({ type: 'error', message: `Update failed: ${e}` });
      setUpdating(false);
      setUpdateStatus('');
    }
  };

  const handleClearAllCaches = async () => {
    setClearingCache(true);
    try {
      const result = await clearAllCaches();
      addToast({ type: 'success', message: result.message });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to clear caches: ${e}` });
    } finally {
      setClearingCache(false);
    }
  };

  // Theme options
  const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor },
  ];

  return (
    <div className="p-6 h-full overflow-y-auto">
      <header className="mb-6">
        <h2 className="text-2xl font-bold flex items-center gap-2">
          <Settings size={24} />
          Settings
        </h2>
        <p className="text-content-secondary">Application settings and maintenance</p>
      </header>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Workspace & General Settings */}
        <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
          <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
                <FolderOpen size={20} className="text-blue-500" />
              </div>
              <div>
                <h3 className="font-semibold">Workspace & General</h3>
                <p className="text-sm text-content-secondary">Manage projects and integrations</p>
              </div>
            </div>
          </div>

          <div className="p-4 space-y-6">
            <div>
              <label className="block text-sm font-medium mb-3">Workspace Directory</label>
              <div className="flex gap-3">
                <input
                  type="text"
                  value={workspacePath}
                  readOnly
                  placeholder="e.g. D:\MyProjects"
                  className="flex-1 px-4 py-2 border border-edge bg-surface-inset rounded-lg text-sm text-content-secondary min-w-0"
                />
                <button
                  onClick={handleChangeWorkspace}
                  disabled={workspaceLoading}
                  className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors flex items-center justify-center min-w-[120px]"
                >
                  {workspaceLoading ? <Loader2 size={16} className="animate-spin" /> : 'Change Folder'}
                </button>
              </div>
              <p className="text-xs text-content-muted mt-2">
                Orbit will create new projects inside this directory by default. Existing projects are not moved.
              </p>
            </div>

            <div className="border-t border-edge-subtle pt-4">
              <label className="block text-sm font-medium mb-3">Ngrok Auth Token</label>
              <div className="flex gap-3">
                <input
                  type="password"
                  value={ngrokToken}
                  onChange={(e) => setNgrokToken(e.target.value)}
                  placeholder="e.g. 2sJ3k9sP2..."
                  className="flex-1 px-4 py-2 border border-edge bg-surface-inset rounded-lg text-sm text-content-secondary min-w-0"
                />
                <button
                  onClick={handleSaveNgrokToken}
                  disabled={ngrokTokenSaving || !ngrokToken}
                  className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors flex items-center justify-center min-w-[120px]"
                >
                  {ngrokTokenSaving ? <Loader2 size={16} className="animate-spin" /> : 'Save Token'}
                </button>
              </div>
              <p className="text-xs text-content-muted mt-2 block">
                Required for sharing local endpoints via Tunnel. Get yours from the{' '}
                <button onClick={() => openUrl("https://dashboard.ngrok.com/get-started/your-authtoken")} className="text-blue-400 hover:underline">
                  Ngrok Dashboard
                </button>.
              </p>
            </div>
          </div>
        </section>

        {/* Appearance + Maintenance (combined column) */}
        <div className="space-y-6">
          {/* Appearance Settings */}
          <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
            <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-lg bg-purple-500/20 flex items-center justify-center">
                  <Palette size={20} className="text-purple-500" />
                </div>
                <div>
                  <h3 className="font-semibold">Appearance</h3>
                  <p className="text-sm text-content-secondary">Customize the look and feel</p>
                </div>
              </div>
            </div>

            <div className="p-4">
              <label className="block text-sm font-medium mb-3">Theme</label>
              <div className="flex gap-3">
                {themeOptions.map((option) => {
                  const Icon = option.icon;
                  const isSelected = theme === option.value;
                  return (
                    <button
                      key={option.value}
                      onClick={() => setTheme(option.value)}
                      className={`flex-1 flex flex-col items-center gap-2 p-4 rounded-xl border-2 transition-all ${isSelected
                        ? 'border-emerald-500 bg-emerald-500/10'
                        : 'border-edge hover:border-edge'
                        }`}
                    >
                      <Icon size={24} className={isSelected ? 'text-emerald-500' : 'text-content-secondary'} />
                      <span className={`text-sm font-medium ${isSelected ? 'text-emerald-500' : ''}`}>
                        {option.label}
                      </span>
                    </button>
                  );
                })}
              </div>
              <p className="text-xs text-content-muted mt-2">
                Current: {resolvedTheme === 'dark' ? 'Dark' : 'Light'} mode
                {theme === 'system' && ' (following system preference)'}
              </p>
            </div>
          </section>

          {/* Maintenance */}
          <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
            <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-lg bg-red-500/20 flex items-center justify-center">
                  <Eraser size={20} className="text-red-500" />
                </div>
                <div>
                  <h3 className="font-semibold">Maintenance</h3>
                  <p className="text-sm text-content-secondary">Clear caches and temporary files</p>
                </div>
              </div>
            </div>

            <div className="p-4">
              <div className="flex items-center justify-between p-4 bg-surface-inset rounded-lg border border-edge-subtle">
                <div>
                  <h4 className="font-medium">Clear All Caches</h4>
                  <p className="text-xs text-content-muted">OPcache, temp files, Nginx cache</p>
                </div>
                <button
                  onClick={handleClearAllCaches}
                  disabled={clearingCache}
                  className="flex items-center gap-2 px-4 py-2 bg-red-600 hover:bg-red-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors"
                >
                  {clearingCache ? <Loader2 size={14} className="animate-spin" /> : <Trash2 size={14} />}
                  Clear Caches
                </button>
              </div>
            </div>
          </section>
        </div>

        {/* System & Advanced Tools */}
        <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
          <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-red-500/20 flex items-center justify-center">
                <Terminal size={20} className="text-red-500" />
              </div>
              <div>
                <h3 className="font-semibold">System Variables</h3>
                <p className="text-sm text-content-secondary">Manage global environment logic</p>
              </div>
            </div>
          </div>

          <div className="p-4 grid grid-cols-2 gap-4">
            <button
              onClick={() => setShowHostsEditor(true)}
              className="flex flex-col items-start gap-2 p-4 bg-surface-inset hover:bg-hover border border-edge rounded-xl transition-colors text-left cursor-pointer"
            >
              <Network size={24} className="text-emerald-500 mb-1" />
              <div className="font-medium">Hosts File Editor</div>
              <div className="text-xs text-content-muted">Override system-level local DNS and configure custom routing mappings.</div>
            </button>

            <button
              onClick={() => setShowPathEditor(true)}
              className="flex flex-col items-start gap-2 p-4 bg-surface-inset hover:bg-hover border border-edge rounded-xl transition-colors text-left cursor-pointer"
            >
              <Zap size={24} className="text-blue-500 mb-1" />
              <div className="font-medium">OS PATH Editor</div>
              <div className="text-xs text-content-muted">Directly augment or manipulate your executable search environment values.</div>
            </button>
          </div>
        </section>

        {/* About & Updates (merged) */}
        <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
          <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-neutral-500/20 flex items-center justify-center">
                <Info size={20} className="text-content-secondary" />
              </div>
              <div>
                <h3 className="font-semibold">About Orbit</h3>
                <p className="text-sm text-content-secondary">Version, updates and license</p>
              </div>
            </div>
          </div>

          <div className="p-4 space-y-4">
            {/* Logo, Version & Update */}
            <div className="flex items-center gap-4 p-4 bg-surface-inset rounded-lg border border-edge-subtle">
              <div className="w-16 h-16 rounded-xl bg-gradient-to-br from-emerald-500 to-emerald-700 flex items-center justify-center flex-shrink-0">
                <svg width="40" height="40" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <circle cx="12" cy="12" r="10" stroke="white" strokeWidth="2" />
                  <circle cx="12" cy="12" r="4" fill="white" />
                  <circle cx="12" cy="4" r="2" fill="white" />
                </svg>
              </div>
              <div className="flex-1 min-w-0">
                <h4 className="text-xl font-bold">Orbit</h4>
                <p className="text-sm text-content-secondary">Modern Local Development Environment</p>
                <div className="flex items-center gap-3 mt-2 flex-wrap">
                  <span className="text-xs bg-emerald-500/20 text-emerald-400 px-2 py-1 rounded font-mono">
                    v{appVersion || '0.1.0'}
                  </span>
                  <span className="text-xs text-content-muted">
                    Tauri {tauriVersion || '2.x'}
                  </span>
                  {!checkingUpdate && !updateAvailable && (
                    <span className="text-xs text-emerald-400 flex items-center gap-1">
                      <CheckCircle size={12} /> Up to date
                    </span>
                  )}
                </div>
              </div>
              <button
                onClick={handleCheckForUpdates}
                disabled={checkingUpdate}
                className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-surface hover:bg-hover border border-edge rounded-lg transition-colors disabled:opacity-50 cursor-pointer flex-shrink-0"
                title="Check for updates"
              >
                {checkingUpdate ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
                {checkingUpdate ? 'Checking...' : 'Check Updates'}
              </button>
            </div>

            {/* Update Banner (inline) */}
            {updateAvailable && (
              <div className="flex items-center gap-4 p-4 bg-gradient-to-r from-emerald-500/10 via-emerald-500/5 to-transparent rounded-lg border border-emerald-500/30">
                <div className="w-10 h-10 rounded-lg bg-emerald-500/20 flex items-center justify-center flex-shrink-0">
                  <Sparkles size={20} className="text-emerald-400" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-0.5">
                    <span className="font-semibold text-sm text-emerald-400">New version available</span>
                    <span className="text-xs bg-emerald-500/20 text-emerald-400 px-2 py-0.5 rounded-full font-mono">
                      v{updateAvailable.version}
                    </span>
                  </div>
                  {updateAvailable.body && (
                    <p className="text-xs text-content-muted truncate">{updateAvailable.body.split('\n')[0]}</p>
                  )}
                  {updating && (
                    <div className="mt-2">
                      <div className="flex items-center justify-between text-xs text-content-muted mb-1">
                        <span>{updateStatus}</span>
                        <span>{updateProgress}%</span>
                      </div>
                      <div className="w-full bg-surface-inset rounded-full h-1.5 overflow-hidden">
                        <div
                          className="bg-emerald-500 h-full rounded-full transition-all duration-300 ease-out"
                          style={{ width: `${updateProgress}%` }}
                        />
                      </div>
                    </div>
                  )}
                </div>
                {!updating ? (
                  <button
                    onClick={handleInstallUpdate}
                    className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors flex-shrink-0 cursor-pointer"
                  >
                    <ArrowUpCircle size={16} />
                    Install & Restart
                  </button>
                ) : (
                  <div className="flex items-center gap-2 px-4 py-2 text-sm text-content-muted flex-shrink-0">
                    <RotateCcw size={16} className="animate-spin" />
                  </div>
                )}
              </div>
            )}

            {/* Links */}
            <div className="grid grid-cols-2 gap-3">
              <button
                onClick={() => openUrl('https://github.com/alinsgit/orbit')}
                className="flex items-center gap-3 p-3 bg-surface-inset rounded-lg border border-edge-subtle hover:border-edge transition-colors text-left cursor-pointer"
              >
                <Github size={18} className="text-content-secondary" />
                <div>
                  <div className="text-sm font-medium">GitHub</div>
                  <div className="text-xs text-content-muted">Source code</div>
                </div>
                <ExternalLink size={14} className="ml-auto text-content-muted" />
              </button>

              <button
                onClick={() => openUrl('https://github.com/alinsgit/orbit/issues')}
                className="flex items-center gap-3 p-3 bg-surface-inset rounded-lg border border-edge-subtle hover:border-edge transition-colors text-left cursor-pointer"
              >
                <Heart size={18} className="text-red-400" />
                <div>
                  <div className="text-sm font-medium">Report Issue</div>
                  <div className="text-xs text-content-muted">Bugs & feedback</div>
                </div>
                <ExternalLink size={14} className="ml-auto text-content-muted" />
              </button>
            </div>

            {/* License */}
            <div className="text-center text-xs text-content-muted pt-2 border-t border-edge-subtle">
              <p>Released under the MIT License</p>
              <p className="mt-1">Copyright 2025 Orbit Dev Team</p>
            </div>
          </div>
        </section>
      </div>

      {showPathEditor && (
        <PathEditorModal onClose={() => setShowPathEditor(false)} />
      )}
      
      {showHostsEditor && (
        <HostsEditorModal onClose={() => setShowHostsEditor(false)} />
      )}
    </div>
  );
}
