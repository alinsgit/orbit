import { useState, useEffect } from 'react';
import { Settings, Loader2, Sun, Moon, Monitor, Palette, FileDown, FileUp, Trash2, Eraser, Shield, Download, CheckCircle, Lock, RefreshCw, Info, ExternalLink, Github, Heart } from 'lucide-react';
import { exportSites, importSites, SiteExport, clearAllCaches, getSslStatus, installMkcert, installSslCa, SslStatus } from '../lib/api';
import { useApp } from '../lib/AppContext';
import { useTheme, Theme } from '../lib/ThemeContext';
import { getVersion, getTauriVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';

export function SettingsManager() {
  const { addToast } = useApp();
  const { theme, setTheme, resolvedTheme } = useTheme();

  // Export/Import State
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);

  // Cache Clear State
  const [clearingCache, setClearingCache] = useState(false);

  // SSL State
  const [sslStatus, setSslStatus] = useState<SslStatus | null>(null);
  const [sslLoading, setSslLoading] = useState(false);

  // About State
  const [appVersion, setAppVersion] = useState<string>('');
  const [tauriVersion, setTauriVersion] = useState<string>('');

  useEffect(() => {
    loadSslStatus();
    loadVersionInfo();
  }, []);

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

  const loadSslStatus = async () => {
    try {
      const status = await getSslStatus();
      setSslStatus(status);
    } catch (e) {
      console.error('Failed to load SSL status:', e);
    }
  };

  const handleInstallMkcert = async () => {
    setSslLoading(true);
    try {
      const result = await installMkcert();
      addToast({ type: 'success', message: result });
      await loadSslStatus();
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to install mkcert: ${e}` });
    } finally {
      setSslLoading(false);
    }
  };

  const handleInstallCa = async () => {
    setSslLoading(true);
    try {
      const result = await installSslCa();
      addToast({ type: 'success', message: result });
      await loadSslStatus();
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to install CA: ${e}` });
    } finally {
      setSslLoading(false);
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

  // Export Sites
  const handleExportSites = async () => {
    setExporting(true);
    try {
      const exportData = await exportSites();

      const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `orbit-sites-${new Date().toISOString().split('T')[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      addToast({ type: 'success', message: `Exported ${exportData.sites.length} sites successfully` });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to export sites: ${e}` });
    } finally {
      setExporting(false);
    }
  };

  // Import Sites
  const handleImportSites = async () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      setImporting(true);
      try {
        const text = await file.text();
        const data: SiteExport = JSON.parse(text);

        if (!data.sites || !Array.isArray(data.sites)) {
          throw new Error('Invalid export file format');
        }

        const result = await importSites(data, true);

        if (result.errors.length > 0) {
          addToast({
            type: 'warning',
            message: `Imported ${result.imported} sites, skipped ${result.skipped}. Errors: ${result.errors.length}`
          });
        } else {
          addToast({
            type: 'success',
            message: `Successfully imported ${result.imported} sites${result.skipped > 0 ? `, skipped ${result.skipped} existing` : ''}`
          });
        }
      } catch (e: any) {
        addToast({ type: 'error', message: `Failed to import sites: ${e.message || e}` });
      } finally {
        setImporting(false);
      }
    };
    input.click();
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

      <div className="max-w-4xl space-y-6">
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

          <div className="p-4 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-3">Theme</label>
              <div className="flex gap-3">
                {themeOptions.map((option) => {
                  const Icon = option.icon;
                  const isSelected = theme === option.value;
                  return (
                    <button
                      key={option.value}
                      onClick={() => setTheme(option.value)}
                      className={`flex-1 flex flex-col items-center gap-2 p-4 rounded-xl border-2 transition-all ${
                        isSelected
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
          </div>
        </section>

        {/* SSL Setup */}
        <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
          <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                <Shield size={20} className="text-emerald-500" />
              </div>
              <div>
                <h3 className="font-semibold">SSL Setup</h3>
                <p className="text-sm text-content-secondary">Install mkcert for local HTTPS</p>
              </div>
            </div>
            <button
              onClick={loadSslStatus}
              className="p-2 hover:bg-hover rounded-lg transition-colors"
              title="Refresh"
            >
              <RefreshCw size={16} />
            </button>
          </div>

          <div className="p-4 space-y-4">
            {/* mkcert Status */}
            <div className="flex items-center gap-4 p-4 bg-surface-inset rounded-lg border border-edge-subtle">
              <div className={`w-3 h-3 rounded-full ${sslStatus?.mkcert_installed ? 'bg-emerald-500' : 'bg-red-500'}`} />
              <div className="flex-1">
                <div className="font-medium">mkcert</div>
                <div className="text-xs text-content-muted">
                  {sslStatus?.mkcert_installed ? 'Installed and ready' : 'Not installed - required for HTTPS'}
                </div>
              </div>
              {!sslStatus?.mkcert_installed ? (
                <button
                  onClick={handleInstallMkcert}
                  disabled={sslLoading}
                  className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors"
                >
                  {sslLoading ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}
                  Install
                </button>
              ) : (
                <CheckCircle size={20} className="text-emerald-500" />
              )}
            </div>

            {/* Root CA Status */}
            {sslStatus?.mkcert_installed && (
              <div className="flex items-center gap-4 p-4 bg-surface-inset rounded-lg border border-edge-subtle">
                <div className={`w-3 h-3 rounded-full ${sslStatus?.ca_installed ? 'bg-emerald-500' : 'bg-amber-500'}`} />
                <div className="flex-1">
                  <div className="font-medium">Root CA</div>
                  <div className="text-xs text-content-muted">
                    {sslStatus?.ca_installed
                      ? 'Installed - browsers will trust local certs'
                      : 'Not installed - browsers will show warnings'}
                  </div>
                </div>
                {!sslStatus?.ca_installed ? (
                  <button
                    onClick={handleInstallCa}
                    disabled={sslLoading}
                    className="flex items-center gap-2 px-4 py-2 bg-amber-600 hover:bg-amber-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors"
                  >
                    {sslLoading ? <Loader2 size={14} className="animate-spin" /> : <Lock size={14} />}
                    Install CA
                  </button>
                ) : (
                  <CheckCircle size={20} className="text-emerald-500" />
                )}
              </div>
            )}

            <p className="text-xs text-content-muted">
              SSL certificates for sites are generated automatically when you enable HTTPS in site settings.
            </p>
          </div>
        </section>

        {/* Export/Import Sites */}
        <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
          <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-blue-500/20 flex items-center justify-center">
                <FileDown size={20} className="text-blue-500" />
              </div>
              <div>
                <h3 className="font-semibold">Backup & Restore</h3>
                <p className="text-sm text-content-secondary">Export and import your site configurations</p>
              </div>
            </div>
          </div>

          <div className="p-4">
            <div className="grid grid-cols-2 gap-4">
              {/* Export */}
              <div className="p-4 bg-surface-inset rounded-lg border border-edge-subtle">
                <div className="flex items-center gap-3 mb-3">
                  <FileDown size={20} className="text-emerald-500" />
                  <h4 className="font-medium">Export Sites</h4>
                </div>
                <p className="text-sm text-content-secondary mb-4">
                  Download all site configurations as JSON.
                </p>
                <button
                  onClick={handleExportSites}
                  disabled={exporting}
                  className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors"
                >
                  {exporting ? <Loader2 size={16} className="animate-spin" /> : <FileDown size={16} />}
                  Export
                </button>
              </div>

              {/* Import */}
              <div className="p-4 bg-surface-inset rounded-lg border border-edge-subtle">
                <div className="flex items-center gap-3 mb-3">
                  <FileUp size={20} className="text-blue-500" />
                  <h4 className="font-medium">Import Sites</h4>
                </div>
                <p className="text-sm text-content-secondary mb-4">
                  Restore from a previously exported file.
                </p>
                <button
                  onClick={handleImportSites}
                  disabled={importing}
                  className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 rounded-lg text-sm font-medium transition-colors"
                >
                  {importing ? <Loader2 size={16} className="animate-spin" /> : <FileUp size={16} />}
                  Import
                </button>
              </div>
            </div>
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

        {/* About */}
        <section className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
          <div className="p-4 border-b border-edge-subtle flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-lg bg-neutral-500/20 flex items-center justify-center">
                <Info size={20} className="text-content-secondary" />
              </div>
              <div>
                <h3 className="font-semibold">About Orbit</h3>
                <p className="text-sm text-content-secondary">Version and license information</p>
              </div>
            </div>
          </div>

          <div className="p-4 space-y-4">
            {/* Logo and Version */}
            <div className="flex items-center gap-4 p-4 bg-surface-inset rounded-lg border border-edge-subtle">
              <div className="w-16 h-16 rounded-xl bg-gradient-to-br from-emerald-500 to-emerald-700 flex items-center justify-center">
                <svg width="40" height="40" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <circle cx="12" cy="12" r="10" stroke="white" strokeWidth="2"/>
                  <circle cx="12" cy="12" r="4" fill="white"/>
                  <circle cx="12" cy="4" r="2" fill="white"/>
                </svg>
              </div>
              <div>
                <h4 className="text-xl font-bold">Orbit</h4>
                <p className="text-sm text-content-secondary">Modern Local Development Environment</p>
                <div className="flex items-center gap-3 mt-2">
                  <span className="text-xs bg-emerald-500/20 text-emerald-400 px-2 py-1 rounded font-mono">
                    v{appVersion || '0.1.0'}
                  </span>
                  <span className="text-xs text-content-muted">
                    Tauri {tauriVersion || '2.x'}
                  </span>
                </div>
              </div>
            </div>

            {/* Links */}
            <div className="grid grid-cols-2 gap-3">
              <button
                onClick={() => open('https://github.com/nicepkg/orbit')}
                className="flex items-center gap-3 p-3 bg-surface-inset rounded-lg border border-edge-subtle hover:border-edge transition-colors text-left"
              >
                <Github size={18} className="text-content-secondary" />
                <div>
                  <div className="text-sm font-medium">GitHub</div>
                  <div className="text-xs text-content-muted">Source code</div>
                </div>
                <ExternalLink size={14} className="ml-auto text-content-muted" />
              </button>

              <button
                onClick={() => open('https://github.com/nicepkg/orbit/issues')}
                className="flex items-center gap-3 p-3 bg-surface-inset rounded-lg border border-edge-subtle hover:border-edge transition-colors text-left"
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
    </div>
  );
}
