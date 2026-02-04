import { useState, useEffect } from 'react';
import { Database, RefreshCw, AlertCircle, CheckCircle, Server, Play, Trash2, Wrench, Settings } from 'lucide-react';
import {
  getDatabaseToolsStatus,
  installAdminer,
  uninstallAdminer,
  setupAdminerNginx,
  removeAdminerNginx,
  installPhpMyAdmin,
  uninstallPhpMyAdmin,
  setupPhpMyAdminNginx,
  removePhpMyAdminNginx,
  getServiceStatus,
  startService,
  getInstalledServices,
  nginxReload,
  type DatabaseToolsStatus,
  type InstalledService,
} from '../lib/api';
import NativeDatabaseManager from './database/NativeDatabaseManager';

type MainTab = 'manage' | 'tools';
type DatabaseTool = 'adminer' | 'phpmyadmin';

export default function DatabaseViewer() {
  const [mainTab, setMainTab] = useState<MainTab>('manage');
  const [status, setStatus] = useState<DatabaseToolsStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<DatabaseTool | null>(null);
  const [uninstalling, setUninstalling] = useState<DatabaseTool | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [mariadbRunning, setMariadbRunning] = useState(false);
  const [phpRunning, setPhpRunning] = useState(false);
  const [nginxRunning, setNginxRunning] = useState(false);
  const [showTool, setShowTool] = useState<DatabaseTool | null>(null);
  const [services, setServices] = useState<InstalledService[]>([]);

  useEffect(() => {
    loadStatus();
  }, []);

  const loadStatus = async () => {
    try {
      setLoading(true);
      setError(null);

      const [toolsStatus, installedServices] = await Promise.all([
        getDatabaseToolsStatus(),
        getInstalledServices(),
      ]);

      setStatus(toolsStatus);
      setServices(installedServices);

      // Check service statuses
      const mariadbService = installedServices.find(s => s.service_type === 'mariadb');
      const phpService = installedServices.find(s => s.service_type.startsWith('php'));
      const nginxService = installedServices.find(s => s.service_type === 'nginx');

      if (mariadbService) {
        try {
          const mariadbStatus = await getServiceStatus(mariadbService.name);
          setMariadbRunning(mariadbStatus === 'running');
        } catch {
          setMariadbRunning(false);
        }
      }

      if (phpService) {
        try {
          const phpStatus = await getServiceStatus(phpService.name);
          setPhpRunning(phpStatus === 'running');
        } catch {
          setPhpRunning(false);
        }
      }

      if (nginxService) {
        try {
          const nginxStatus = await getServiceStatus(nginxService.name);
          setNginxRunning(nginxStatus === 'running');
        } catch {
          setNginxRunning(false);
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load status');
    } finally {
      setLoading(false);
    }
  };

  const getPhpPort = () => {
    const phpService = services.find(s => s.service_type === 'php' || s.service_type.startsWith('php'));
    if (phpService) {
      const versionMatch = phpService.name.match(/php-(\d+)\.(\d+)/);
      if (versionMatch) {
        const major = parseInt(versionMatch[1]);
        const minor = parseInt(versionMatch[2]);
        if (major === 8) {
          return 9000 + minor;
        } else if (major === 7) {
          return 9070 + minor;
        }
      }
    }
    return 9004;
  };

  const handleInstallAdminer = async () => {
    try {
      setInstalling('adminer');
      setError(null);
      await installAdminer();
      await setupAdminerNginx(getPhpPort());

      if (nginxRunning) {
        try {
          await nginxReload();
        } catch (reloadErr) {
          console.warn('Failed to reload nginx:', reloadErr);
        }
      }

      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to install Adminer');
    } finally {
      setInstalling(null);
    }
  };

  const handleUninstallAdminer = async () => {
    try {
      setUninstalling('adminer');
      setError(null);
      await removeAdminerNginx();
      await uninstallAdminer();

      if (nginxRunning) {
        try {
          await nginxReload();
        } catch (reloadErr) {
          console.warn('Failed to reload nginx:', reloadErr);
        }
      }

      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to uninstall Adminer');
    } finally {
      setUninstalling(null);
    }
  };

  const handleInstallPhpMyAdmin = async () => {
    try {
      setInstalling('phpmyadmin');
      setError(null);
      await installPhpMyAdmin();
      await setupPhpMyAdminNginx(getPhpPort());

      if (nginxRunning) {
        try {
          await nginxReload();
        } catch (reloadErr) {
          console.warn('Failed to reload nginx:', reloadErr);
        }
      }

      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to install PhpMyAdmin');
    } finally {
      setInstalling(null);
    }
  };

  const handleUninstallPhpMyAdmin = async () => {
    try {
      setUninstalling('phpmyadmin');
      setError(null);
      await removePhpMyAdminNginx();
      await uninstallPhpMyAdmin();

      if (nginxRunning) {
        try {
          await nginxReload();
        } catch (reloadErr) {
          console.warn('Failed to reload nginx:', reloadErr);
        }
      }

      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to uninstall PhpMyAdmin');
    } finally {
      setUninstalling(null);
    }
  };

  const handleStartServices = async () => {
    try {
      setError(null);

      const mariadbService = services.find(s => s.service_type === 'mariadb');
      const phpService = services.find(s => s.service_type.startsWith('php'));
      const nginxService = services.find(s => s.service_type === 'nginx');

      if (mariadbService && !mariadbRunning) {
        await startService(mariadbService.name, mariadbService.path);
      }
      if (phpService && !phpRunning) {
        await startService(phpService.name, phpService.path);
      }
      if (nginxService && !nginxRunning) {
        await startService(nginxService.name, nginxService.path);
      }

      await new Promise(resolve => setTimeout(resolve, 2000));
      await loadStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start services');
    }
  };

  const handleOpenTool = (tool: DatabaseTool) => {
    if (allServicesRunning) {
      setShowTool(tool);
    }
  };

  const allServicesRunning = mariadbRunning && phpRunning && nginxRunning;
  const hasRequiredServices = services.some(s => s.service_type === 'mariadb') &&
    services.some(s => s.service_type.startsWith('php')) &&
    services.some(s => s.service_type === 'nginx');

  if (showTool && status) {
    const toolUrl = showTool === 'adminer' ? status.adminer.adminer_url : status.phpmyadmin.url;
    const toolName = showTool === 'adminer' ? 'Adminer' : 'PhpMyAdmin';

    return (
      <div className="flex flex-col h-full bg-surface">
        <div className="flex items-center justify-between p-3 border-b border-edge bg-surface">
          <div className="flex items-center gap-2">
            <Database className="w-5 h-5 text-orange-500" />
            <span className="font-medium text-content">{toolName}</span>
          </div>
          <button
            onClick={() => setShowTool(null)}
            className="px-3 py-1.5 text-sm bg-surface-raised hover:bg-hover rounded-lg transition-colors"
          >
            Back
          </button>
        </div>
        <div className="flex-1 bg-white">
          <iframe
            src={toolUrl}
            className="w-full h-full border-0"
            title={toolName}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header with main tabs */}
      <div className="p-4 border-b border-edge bg-surface-inset">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Database className="w-8 h-8 text-orange-500" />
            <div>
              <h2 className="text-xl font-semibold text-content">Database</h2>
              <p className="text-sm text-content-secondary">Manage your MariaDB databases</p>
            </div>
          </div>
        </div>

        {/* Main Tabs */}
        <div className="flex gap-2">
          <button
            onClick={() => setMainTab('manage')}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              mainTab === 'manage'
                ? 'bg-emerald-600 text-white'
                : 'bg-surface-raised text-content-secondary hover:text-content'
            }`}
          >
            <Wrench className="w-4 h-4" />
            Quick Manage
          </button>
          <button
            onClick={() => setMainTab('tools')}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              mainTab === 'tools'
                ? 'bg-orange-600 text-white'
                : 'bg-surface-raised text-content-secondary hover:text-content'
            }`}
          >
            <Settings className="w-4 h-4" />
            Web Tools
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {mainTab === 'manage' ? (
          <NativeDatabaseManager />
        ) : (
          <div className="p-6 h-full overflow-y-auto space-y-6">
            {/* Refresh button */}
            <div className="flex justify-end">
              <button
                onClick={loadStatus}
                disabled={loading}
                className="p-2 hover:bg-surface-raised rounded-lg transition-colors disabled:opacity-50"
              >
                <RefreshCw className={`w-5 h-5 ${loading ? 'animate-spin' : ''}`} />
              </button>
            </div>

            {error && (
              <div className="flex items-center gap-2 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400">
                <AlertCircle className="w-5 h-5 flex-shrink-0" />
                <span className="text-sm">{error}</span>
              </div>
            )}

            {/* Service Status */}
            <div className="bg-surface-raised rounded-xl p-4 space-y-3">
              <h3 className="text-sm font-medium text-content-secondary mb-3">Required Services</h3>

              <div className="grid grid-cols-3 gap-3">
                <div className={`p-3 rounded-lg ${mariadbRunning ? 'bg-green-500/10 border border-green-500/20' : 'bg-surface-raised border border-edge'}`}>
                  <div className="flex items-center gap-2">
                    <Server className={`w-4 h-4 ${mariadbRunning ? 'text-green-500' : 'text-content-muted'}`} />
                    <span className="text-sm font-medium">MariaDB</span>
                  </div>
                  <p className={`text-xs mt-1 ${mariadbRunning ? 'text-green-400' : 'text-content-muted'}`}>
                    {mariadbRunning ? 'Running' : 'Stopped'}
                  </p>
                </div>

                <div className={`p-3 rounded-lg ${phpRunning ? 'bg-green-500/10 border border-green-500/20' : 'bg-surface-raised border border-edge'}`}>
                  <div className="flex items-center gap-2">
                    <Server className={`w-4 h-4 ${phpRunning ? 'text-green-500' : 'text-content-muted'}`} />
                    <span className="text-sm font-medium">PHP</span>
                  </div>
                  <p className={`text-xs mt-1 ${phpRunning ? 'text-green-400' : 'text-content-muted'}`}>
                    {phpRunning ? 'Running' : 'Stopped'}
                  </p>
                </div>

                <div className={`p-3 rounded-lg ${nginxRunning ? 'bg-green-500/10 border border-green-500/20' : 'bg-surface-raised border border-edge'}`}>
                  <div className="flex items-center gap-2">
                    <Server className={`w-4 h-4 ${nginxRunning ? 'text-green-500' : 'text-content-muted'}`} />
                    <span className="text-sm font-medium">Nginx</span>
                  </div>
                  <p className={`text-xs mt-1 ${nginxRunning ? 'text-green-400' : 'text-content-muted'}`}>
                    {nginxRunning ? 'Running' : 'Stopped'}
                  </p>
                </div>
              </div>

              {hasRequiredServices && !allServicesRunning && (
                <button
                  onClick={handleStartServices}
                  className="w-full mt-3 flex items-center justify-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 rounded-lg transition-colors"
                >
                  <Play className="w-4 h-4" />
                  Start Required Services
                </button>
              )}

              {!hasRequiredServices && (
                <p className="text-sm text-amber-400 mt-3">
                  Please install MariaDB, PHP, and Nginx from the Services tab first.
                </p>
              )}
            </div>

            {/* Database Tools */}
            <div className="space-y-4">
              <h3 className="text-sm font-medium text-content-secondary">Web-Based Database Tools</h3>

              {/* Adminer */}
              <div className="bg-surface-raised rounded-xl p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <h4 className="text-sm font-medium text-content">Adminer</h4>
                      {status?.adminer.adminer_installed && (
                        <CheckCircle className="w-4 h-4 text-green-500" />
                      )}
                    </div>
                    <p className="text-xs text-content-muted mt-1">
                      Lightweight single-file database manager (Port 8080)
                    </p>
                  </div>

                  <div className="flex items-center gap-2">
                    {status?.adminer.adminer_installed ? (
                      <>
                        <button
                          onClick={() => handleOpenTool('adminer')}
                          disabled={!allServicesRunning}
                          className={`px-4 py-2 rounded-lg transition-colors text-sm font-medium ${
                            allServicesRunning
                              ? 'bg-orange-600 hover:bg-orange-700'
                              : 'bg-surface-inset cursor-not-allowed opacity-50'
                          }`}
                        >
                          Open
                        </button>
                        <button
                          onClick={handleUninstallAdminer}
                          disabled={uninstalling === 'adminer'}
                          className="p-2 text-red-400 hover:bg-red-500/10 rounded-lg transition-colors disabled:opacity-50"
                          title="Uninstall Adminer"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </>
                    ) : (
                      <button
                        onClick={handleInstallAdminer}
                        disabled={installing === 'adminer' || !hasRequiredServices}
                        className="px-4 py-2 bg-orange-600 hover:bg-orange-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors text-sm font-medium"
                      >
                        {installing === 'adminer' ? 'Installing...' : 'Install'}
                      </button>
                    )}
                  </div>
                </div>
              </div>

              {/* PhpMyAdmin */}
              <div className="bg-surface-raised rounded-xl p-4">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <h4 className="text-sm font-medium text-content">PhpMyAdmin</h4>
                      {status?.phpmyadmin.installed && (
                        <CheckCircle className="w-4 h-4 text-green-500" />
                      )}
                    </div>
                    <p className="text-xs text-content-muted mt-1">
                      Full-featured database administration tool (Port 8081)
                    </p>
                  </div>

                  <div className="flex items-center gap-2">
                    {status?.phpmyadmin.installed ? (
                      <>
                        <button
                          onClick={() => handleOpenTool('phpmyadmin')}
                          disabled={!allServicesRunning}
                          className={`px-4 py-2 rounded-lg transition-colors text-sm font-medium ${
                            allServicesRunning
                              ? 'bg-blue-600 hover:bg-blue-700'
                              : 'bg-surface-inset cursor-not-allowed opacity-50'
                          }`}
                        >
                          Open
                        </button>
                        <button
                          onClick={handleUninstallPhpMyAdmin}
                          disabled={uninstalling === 'phpmyadmin'}
                          className="p-2 text-red-400 hover:bg-red-500/10 rounded-lg transition-colors disabled:opacity-50"
                          title="Uninstall PhpMyAdmin"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </>
                    ) : (
                      <button
                        onClick={handleInstallPhpMyAdmin}
                        disabled={installing === 'phpmyadmin' || !hasRequiredServices}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors text-sm font-medium"
                      >
                        {installing === 'phpmyadmin' ? 'Installing...' : 'Install'}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Connection Info */}
            {(status?.adminer.adminer_installed || status?.phpmyadmin.installed) && (
              <div className="bg-surface-raised rounded-xl p-4">
                <h3 className="text-sm font-medium text-content-secondary mb-3">Connection Information</h3>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-content-muted">Host:</span>
                    <code className="text-content-secondary bg-surface px-2 py-0.5 rounded">127.0.0.1</code>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-content-muted">Port:</span>
                    <code className="text-content-secondary bg-surface px-2 py-0.5 rounded">3306</code>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-content-muted">Default User:</span>
                    <code className="text-content-secondary bg-surface px-2 py-0.5 rounded">root</code>
                  </div>
                  {status?.adminer.adminer_installed && (
                    <div className="flex justify-between">
                      <span className="text-content-muted">Adminer URL:</span>
                      <code className="text-content-secondary bg-surface px-2 py-0.5 rounded text-xs">{status.adminer.adminer_url}</code>
                    </div>
                  )}
                  {status?.phpmyadmin.installed && (
                    <div className="flex justify-between">
                      <span className="text-content-muted">PhpMyAdmin URL:</span>
                      <code className="text-content-secondary bg-surface px-2 py-0.5 rounded text-xs">{status.phpmyadmin.url}</code>
                    </div>
                  )}
                </div>

                {!allServicesRunning && (
                  <p className="text-xs text-amber-400 mt-3">
                    Start all required services to access the database tools
                  </p>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
