import { useState, useEffect, useRef, useMemo } from 'react';
import { Download, Loader2, Trash2, RefreshCw, Play, Square, CheckCircle, Terminal, CheckCircle2, RotateCw, Settings2, Zap, ChevronDown, Check } from 'lucide-react';
import {
  getAvailableVersions,
  downloadService,
  uninstallService,
  refreshAllVersions,
  ServiceVersion,
  addServiceToPath,
  removeServiceFromPath,
  checkServicePathStatus,
  ServicePathStatus,
  reloadService,
  listServiceVersions,
  setActiveServiceVersion,
  removeServiceVersion,
} from '../lib/api';
import { useApp } from '../lib/AppContext';
import { settingsStore } from '../lib/store';
import { ServiceConfigDrawer } from './ServiceConfigDrawer';
import { ServiceOverview } from './ServiceOverview';
import { ComposerManager } from './ComposerManager';
import { MailManager } from './MailManager';
import { MeilisearchManager } from './MeilisearchManager';
import { CliManager, CliCommandReference } from './CliManager';
import { LogViewer } from './LogViewer';

import { getServiceIcon } from '../lib/serviceIcons';
import { ask } from '@tauri-apps/plugin-dialog';

export function ServiceManager() {
  const {
    services,
    refreshServices,
    startServiceByName,
    stopServiceByName,
    settings,
    refreshSettings,
    addToast
  } = useApp();

  const [subTab, setSubTab] = useState<'overview' | 'manage' | 'install' | 'tools' | 'logs'>('overview');
  // Service catalog - single source of truth for installable services
  const SERVICE_CATALOG = [
    { key: 'nginx', title: 'Nginx Web Server', icon: '🌐', group: 'server' },
    { key: 'apache', title: 'Apache HTTP Server', icon: '🪶', group: 'server' },
    { key: 'php', title: 'PHP Interpreter', icon: '🐘', group: 'server' },
    { key: 'mariadb', title: 'MariaDB Database', icon: '🗄️', group: 'server' },
    { key: 'postgresql', title: 'PostgreSQL Database', icon: '🐘', group: 'server' },
    { key: 'mongodb', title: 'MongoDB Database', icon: '🍃', group: 'server' },
    { key: 'redis', title: 'Redis Cache Store', icon: '🗝️', group: 'server' },
    { key: 'nodejs', title: 'Node.js Runtime', icon: '💚', group: 'devtools' },
    { key: 'python', title: 'Python', icon: '🐍', group: 'devtools' },
    { key: 'bun', title: 'Bun Runtime', icon: '🥟', group: 'devtools' },
    { key: 'go', title: 'Go (Golang)', icon: '🟦', group: 'devtools' },
    { key: 'deno', title: 'Deno Runtime', icon: '🦕', group: 'devtools' },
    { key: 'rust', title: 'Rust Toolchain', icon: '🦀', group: 'devtools' },
  ] as const;

  const [available, setAvailable] = useState<Record<string, ServiceVersion[]>>({});
  // Per-service-type list of installed version directory names (e.g.
  // {"nginx": ["1.27.3", "1.28.1"]}). The scanner only reports the *active*
  // version for junction-based services, so we need this to know which
  // entries in the registry are already on disk.
  const [installedDirs, setInstalledDirs] = useState<Record<string, string[]>>({});

  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [processing, setProcessing] = useState<string | null>(null);
  const [reloading, setReloading] = useState<string | null>(null);
  const [pathStatuses, setPathStatuses] = useState<Record<string, ServicePathStatus>>({});
  const [pathProcessing, setPathProcessing] = useState<string | null>(null);

  // Config drawer state
  const [configDrawer, setConfigDrawer] = useState<{
    isOpen: boolean;
    serviceName: string;
    serviceType: string;
    serviceVersion: string;
  }>({ isOpen: false, serviceName: '', serviceType: '', serviceVersion: '' });

  // Services that support autostart (have a server process)
  const STARTABLE_TYPES = ['nginx', 'php', 'mariadb', 'redis', 'apache', 'mailpit', 'postgresql', 'mongodb'];

  const isStartable = (serviceType: string) => STARTABLE_TYPES.includes(serviceType);

  const isAutostart = (serviceName: string) =>
    settings.services.autostart_list?.includes(serviceName) ?? false;

  const handleToggleAutostart = async (serviceName: string) => {
    const current = settings.services.autostart_list || [];
    const newList = current.includes(serviceName)
      ? current.filter(n => n !== serviceName)
      : [...current, serviceName];

    await settingsStore.saveSettings({
      services: { ...settings.services, autostart_list: newList },
    });
    await refreshSettings();

    const enabled = newList.includes(serviceName);
    addToast({
      type: 'success',
      message: enabled ? `${serviceName} will start on launch` : `${serviceName} autostart disabled`,
    });
  };

  // Services that have configuration
  const hasConfiguration = (serviceType: string) => {
    return ['php', 'nginx', 'mariadb', 'apache', 'redis'].includes(serviceType);
  };

  const openConfigDrawer = (name: string, type: string, version: string) => {
    setConfigDrawer({ isOpen: true, serviceName: name, serviceType: type, serviceVersion: version });
  };

  const closeConfigDrawer = () => {
    setConfigDrawer({ ...configDrawer, isOpen: false });
  };

  // Check if a version is already installed.
  //
  // Two sources of truth:
  //   1. `services` — what scanner reports (only the *active* version for
  //      junction-based services, all PHP versions).
  //   2. `installedDirs` — directory names under `bin/.versions/<svc>/` and
  //      `bin/php/`. This catches non-active multi-version installs that
  //      scanner doesn't surface.
  const isVersionInstalled = (type: string, version: string) => {
    const available = version.trim();

    // (2) Direct directory match. installedDirs are dir names, which for
    // versioned services come straight from the registry version string
    // we wrote at install time, so an exact match here is reliable.
    const dirs = installedDirs[type];
    if (dirs && dirs.length > 0) {
      if (dirs.includes(available)) return true;
      // Apache registry uses "2.4.66-VS18"; install dir uses raw string.
      // Strip the VS suffix and compare prefix.
      if (type === 'apache') {
        const numeric = available.replace(/-VS\d+$/, '');
        if (dirs.some((d) => d === numeric || d.startsWith(numeric))) return true;
      }
      // Bun/nodejs registry can be more granular than the dir name.
      if (type === 'bun' || type === 'nodejs') {
        const aParts = available.split('.');
        if (
          dirs.some((d) => {
            const iParts = d.split('.');
            if (type === 'nodejs') return iParts[0] === aParts[0];
            return iParts[0] === aParts[0] && iParts[1] === aParts[1];
          })
        ) {
          return true;
        }
      }
    }

    // (1) Fall back to the scanner-derived match logic.
    return services.some(s => {
      if (s.service_type !== type) return false;
      const installed = s.version.trim();

      // Exact match
      if (installed === available) return true;

      const iParts = installed.split('.');
      const aParts = available.split('.');

      // Multi-version: match by available version depth
      // Some services use major-only keys (postgresql "16", mongodb "8.0"), others use major.minor (php "8.4")
      if (['php', 'mariadb', 'postgresql', 'mongodb', 'python', 'nginx', 'go'].includes(type)) {
        // Match all parts that the available (registry) version specifies
        for (let i = 0; i < aParts.length; i++) {
          if (iParts[i] !== aParts[i]) return false;
        }
        return true;
      }

      // Node.js: match major version (22.x.x)
      if (type === 'nodejs') {
        return iParts[0] === aParts[0];
      }

      // Bun: match major.minor (1.3.x)
      if (type === 'bun') {
        return iParts[0] === aParts[0] && iParts[1] === aParts[1];
      }

      // Apache: version key is "2.4.66-VS18", installed is "2.4.66"
      // Match if installed version starts with the available version's numeric part
      if (type === 'apache') {
        const numericAvailable = available.replace(/-VS\d+$/, '');
        return installed === numericAvailable || installed.startsWith(numericAvailable);
      }

      return false;
    });
  };

  const fetchAvailable = async (forceRefresh = false) => {
    setLoading(true);
    try {
      const keys = SERVICE_CATALOG.map(s => s.key);
      const results = await Promise.allSettled(
        keys.map(key => getAvailableVersions(key, forceRefresh))
      );

      const newAvailable: Record<string, ServiceVersion[]> = {};
      results.forEach((result, index) => {
        if (result.status === 'fulfilled') {
          newAvailable[keys[index]] = result.value;
        } else {
          newAvailable[keys[index]] = [];
          console.warn(`Failed to fetch ${keys[index]} versions:`, result.reason);
        }
      });
      setAvailable(newAvailable);
    } catch (e) {
      console.error(e);
      addToast({ type: 'error', message: 'Failed to fetch available versions' });
    } finally {
      setLoading(false);
    }
  };

  const handleRefreshVersions = async () => {
    setRefreshing(true);
    try {
      await refreshAllVersions();
      await Promise.all([fetchAvailable(true), fetchInstalledDirs()]);
      addToast({ type: 'success', message: 'Versions refreshed' });
    } catch (e) {
      console.error(e);
      addToast({ type: 'error', message: 'Failed to refresh versions' });
    } finally {
      setRefreshing(false);
    }
  };

  // Pull every installed version directory for every service in the catalog
  // (also picks up PHP, which uses its own non-junction layout). Used to
  // mark Install-tab cards as "Installed" even when the scanner only sees
  // the active version.
  const fetchInstalledDirs = async () => {
    const keys = SERVICE_CATALOG.map((s) => s.key);
    const results = await Promise.allSettled(
      keys.map((key) => listServiceVersions(key)),
    );
    const next: Record<string, string[]> = {};
    results.forEach((res, i) => {
      const key = keys[i];
      if (res.status === 'fulfilled') {
        next[key] = res.value.installed;
      } else {
        next[key] = [];
      }
    });
    setInstalledDirs(next);
  };

  // Check PATH status for all services
  const checkAllPathStatuses = async () => {
    const statuses: Record<string, ServicePathStatus> = {};
    for (const service of services) {
      try {
        // Use service.name for PHP (e.g., "php-8.4") to get correct path
        const pathServiceType = service.service_type === 'php' ? service.name : service.service_type;
        const status = await checkServicePathStatus(pathServiceType);
        statuses[service.name] = status;
      } catch {
        // Silently ignore - command may not be available yet
      }
    }
    setPathStatuses(statuses);
  };

  const handleTogglePath = async (serviceName: string, serviceType: string, currentlyInPath: boolean) => {
    setPathProcessing(serviceName);
    try {
      if (currentlyInPath) {
        await removeServiceFromPath(serviceType);
        addToast({ type: 'success', message: `${serviceName} removed from PATH. Restart terminal to apply.` });
      } else {
        await addServiceToPath(serviceType);
        addToast({ type: 'success', message: `${serviceName} added to PATH. Restart terminal to apply.` });
      }
      // Update status for this service
      const status = await checkServicePathStatus(serviceType);
      setPathStatuses(prev => ({ ...prev, [serviceName]: status }));
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: `Failed to update PATH: ${e}` });
    } finally {
      setPathProcessing(null);
    }
  };

  useEffect(() => {
    fetchAvailable();
    fetchInstalledDirs();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Refresh the installed-dirs cache whenever scanner-reported services change
  // (e.g. after install / uninstall / version switch). Cheap parallel calls.
  useEffect(() => {
    fetchInstalledDirs();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [services.length]);

  // Check PATH statuses when services change (debounced)
  useEffect(() => {
    if (services.length > 0) {
      const timer = setTimeout(() => {
        checkAllPathStatuses();
      }, 500);
      return () => clearTimeout(timer);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [services.length]);

  const handleInstall = async (serviceType: string, version: ServiceVersion) => {
    setProcessing(`${serviceType}-${version.version}`);
    try {
      const filename = version.filename || `${serviceType}-${version.version}.zip`;

      let typeParam = serviceType;
      if (serviceType === 'php') {
        const majorMinor = version.version.split('.').slice(0, 2).join('.');
        typeParam = `php-${majorMinor}`;
      }

      // Pass the explicit version string so the backend knows where to land
      // it under bin/.versions/<svc>/<ver>/ (irrelevant for PHP, which keeps
      // its own `bin/php/<ver>/` layout, but harmless).
      await downloadService(version.download_url, filename, typeParam, version.version);
      await Promise.all([refreshServices(), fetchInstalledDirs()]);
      addToast({ type: 'success', message: `${serviceType} ${version.version} installed successfully` });
      setSubTab('manage');
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: `Installation failed: ${e}` });
    } finally {
      setProcessing(null);
    }
  };

  const handleUninstall = async (name: string, serviceType: string, path: string) => {
    const confirmed = await ask(`Are you sure you want to uninstall ${name}? This will remove all files.`, { title: 'Confirm Uninstall', kind: 'warning' });
    if (!confirmed) return;

    setProcessing(name);
    try {
      await uninstallService(name, serviceType, path);
      await Promise.all([refreshServices(), fetchInstalledDirs()]);
      addToast({ type: 'success', message: `${name} uninstalled` });
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: `Failed to uninstall: ${e}` });
    } finally {
      setProcessing(null);
    }
  };

  const handleToggleService = async (name: string, status: string) => {
    setProcessing(name);
    try {
      if (status === 'running') {
        await stopServiceByName(name);
      } else {
        await startServiceByName(name);
      }
    } finally {
      setProcessing(null);
    }
  };

  const handleReloadService = async (name: string) => {
    setReloading(name);
    try {
      const message = await reloadService(name);
      addToast({ type: 'success', message });
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: `Reload failed: ${e}` });
    } finally {
      setReloading(null);
    }
  };

  // Services that support reload
  const supportsReload = (serviceType: string) => {
    return serviceType === 'nginx' || serviceType === 'apache';
  };

  // Collapse PHP entries into a single "primary" card so each installed PHP
  // version isn't shown both as its own card AND inside the version dropdown.
  // Picks the first running PHP if any, otherwise the highest version. The
  // remaining versions stay reachable through the picker, with per-row
  // Start/Stop and remove buttons.
  const consolidatedServices = useMemo(() => {
    const phpEntries = services.filter((s) => s.service_type === 'php');
    if (phpEntries.length <= 1) return services;
    const sorted = [...phpEntries].sort((a, b) =>
      b.version.localeCompare(a.version, undefined, { numeric: true }),
    );
    const primary = phpEntries.find((p) => p.status === 'running') || sorted[0];
    const others = services.filter((s) => s.service_type !== 'php');
    // Preserve the original order: drop in primary at the position of the
    // first PHP entry so the card list doesn't reshuffle on every render.
    const firstPhpIdx = services.findIndex((s) => s.service_type === 'php');
    const out = [...others];
    out.splice(firstPhpIdx, 0, primary);
    return out;
  }, [services]);

  // Per-version PHP status map (keyed by directory name "8.4" — matches what
  // list_service_versions returns). Drives Start/Stop buttons in the dropdown.
  const phpStatusMap = useMemo<
    Record<string, { name: string; status: string; port?: number | null }>
  >(() => {
    const m: Record<string, { name: string; status: string; port?: number | null }> = {};
    for (const s of services) {
      if (s.service_type !== 'php') continue;
      const dirVersion = s.name.replace(/^php-/, '');
      m[dirVersion] = { name: s.name, status: s.status, port: s.port };
    }
    return m;
  }, [services]);

  return (
    <div className="p-6 h-full flex flex-col">
      <header className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold">Services</h2>
        <div className="flex items-center gap-3">
        <div className="flex flex-wrap bg-surface-raised p-1 rounded-lg">
          <button
            onClick={() => setSubTab('overview')}
            className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${subTab === 'overview'
              ? 'bg-surface-inset text-content shadow'
              : 'text-content-secondary hover:text-content'
              }`}
          >
            Overview
          </button>
          <button
            onClick={() => setSubTab('manage')}
            className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${subTab === 'manage'
              ? 'bg-surface-inset text-content shadow'
              : 'text-content-secondary hover:text-content'
              }`}
          >
            Manage ({services.length})
          </button>
          <button
            onClick={() => setSubTab('install')}
            className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${subTab === 'install'
              ? 'bg-surface-inset text-content shadow'
              : 'text-content-secondary hover:text-content'
              }`}
          >
            Install
          </button>
          <button
            onClick={() => setSubTab('tools')}
            className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${subTab === 'tools'
              ? 'bg-surface-inset text-content shadow'
              : 'text-content-secondary hover:text-content'
              }`}
          >
            Tools
          </button>
          <button
            onClick={() => setSubTab('logs')}
            className={`px-4 py-2 rounded-md text-sm font-medium transition-all ${subTab === 'logs'
              ? 'bg-surface-inset text-content shadow'
              : 'text-content-secondary hover:text-content'
              }`}
          >
            Logs
          </button>
        </div>
        {(subTab === 'install') && (
          <button
            onClick={handleRefreshVersions}
            disabled={refreshing || loading}
            className="p-2 rounded-lg bg-surface-raised hover:bg-hover text-content-secondary hover:text-content transition-colors disabled:opacity-50"
            title="Refresh versions from server"
          >
            <RefreshCw size={18} className={refreshing ? 'animate-spin' : ''} />
          </button>
        )}
        </div>
      </header>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {subTab === 'overview' && (
          <ServiceOverview onNavigateToInstall={() => setSubTab('install')} />
        )}

        {subTab === 'manage' && (
          <div className="space-y-3">
            {services.length === 0 && (
              <div className="text-center py-12 text-content-muted border-2 border-dashed border-edge rounded-xl">
                <Download size={48} className="mx-auto mb-4 opacity-50" />
                <p className="mb-2">No services installed yet.</p>
                <button
                  onClick={() => setSubTab('install')}
                  className="text-emerald-500 hover:underline"
                >
                  Browse available services
                </button>
              </div>
            )}
            {consolidatedServices.map((service) => (
              <div
                key={service.name}
                className="bg-surface-raised p-4 rounded-xl border border-edge-subtle hover:border-edge transition-all"
              >
                <div className="flex justify-between items-center">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 rounded-lg bg-surface-inset flex items-center justify-center text-2xl">
                      {getServiceIcon(service.service_type)}
                    </div>
                    <div>
                      <h3 className="font-semibold flex items-center gap-2">
                        {service.name}
                        {MULTI_VERSION_SERVICES.has(service.service_type) ? (
                          <VersionPicker
                            serviceType={service.service_type}
                            serviceName={service.name}
                            currentVersion={service.version}
                            serviceStatus={service.status}
                            onChanged={async () => {
                              await refreshServices();
                              await fetchInstalledDirs();
                            }}
                          />
                        ) : service.service_type === 'php' ? (
                          // PHP runs every installed version concurrently on
                          // its own port — there is no "active" PHP. The
                          // picker is shown in info-only mode: lists all
                          // installed PHP versions, exposes per-row Start/
                          // Stop, and lets the user uninstall a specific
                          // version. No switch action.
                          <VersionPicker
                            serviceType="php"
                            currentVersion={service.version}
                            serviceStatus={service.status}
                            onChanged={async () => {
                              await refreshServices();
                              await fetchInstalledDirs();
                            }}
                            disableSwitch
                            versionStatuses={phpStatusMap}
                            onToggleVersion={async (_dirVer, name, currentStatus) => {
                              setProcessing(name);
                              try {
                                if (currentStatus === 'running') {
                                  await stopServiceByName(name);
                                } else {
                                  await startServiceByName(name);
                                }
                              } finally {
                                setProcessing(null);
                              }
                            }}
                          />
                        ) : (
                          <span className="text-xs font-normal text-content-muted bg-surface-raised px-2 py-0.5 rounded">
                            v{service.version}
                          </span>
                        )}
                      </h3>
                      <p className="text-sm text-content-secondary truncate max-w-[300px]">
                        {service.path}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {/* Status indicator */}
                    {!isStartable(service.service_type) ? (
                      <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium bg-blue-500/10 text-blue-400">
                        <div className="w-2 h-2 rounded-full bg-blue-400" />
                        INSTALLED
                      </div>
                    ) : (
                      <div className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium ${service.status === 'running'
                        ? 'bg-emerald-500/10 text-emerald-500'
                        : service.status === 'starting' || service.status === 'stopping'
                          ? 'bg-amber-500/10 text-amber-500'
                          : 'bg-surface-inset text-content-secondary'
                        }`}>
                        <div className={`w-2 h-2 rounded-full ${service.status === 'running'
                          ? 'bg-emerald-500'
                          : service.status === 'starting' || service.status === 'stopping'
                            ? 'bg-amber-500 animate-pulse'
                            : 'bg-content-muted'
                          }`} />
                        {service.status.toUpperCase()}
                        {service.port && <span className="font-mono">:{service.port}</span>}
                      </div>
                    )}

                    {/* Configure button (for services with settings) */}
                    {hasConfiguration(service.service_type) && (
                      <button
                        onClick={() => {
                          // For PHP, extract major.minor from version (e.g., "8.4.5" -> "8.4")
                          const version = service.service_type === 'php'
                            ? service.version.split('.').slice(0, 2).join('.')
                            : service.version;
                          openConfigDrawer(service.name, service.service_type, version);
                        }}
                        className="p-2 rounded-lg bg-surface-inset hover:bg-hover text-content-secondary hover:text-content transition-colors"
                        title="Configure"
                      >
                        <Settings2 size={16} />
                      </button>
                    )}

                    {/* PATH toggle button */}
                    {(() => {
                      const pathStatus = pathStatuses[service.name];
                      const isInPath = pathStatus?.in_path;
                      const isProcessingPath = pathProcessing === service.name;
                      // Use service.name for PHP (e.g., "php-8.4") to get correct path
                      const pathServiceType = service.service_type === 'php' ? service.name : service.service_type;
                      return (
                        <button
                          onClick={() => handleTogglePath(service.name, pathServiceType, !!isInPath)}
                          disabled={isProcessingPath}
                          className={`p-2 rounded-lg transition-colors disabled:opacity-50 ${isInPath
                            ? 'bg-emerald-500/10 text-emerald-500 hover:bg-red-500/10 hover:text-red-500'
                            : 'bg-surface-inset hover:bg-hover text-content-secondary hover:text-content'
                            }`}
                          title={isInPath ? 'Remove from PATH' : 'Add to PATH'}
                        >
                          {isProcessingPath ? (
                            <Loader2 size={16} className="animate-spin" />
                          ) : isInPath ? (
                            <CheckCircle2 size={16} />
                          ) : (
                            <Terminal size={16} />
                          )}
                        </button>
                      );
                    })()}

                    {/* Autostart toggle */}
                    {isStartable(service.service_type) && (
                      <button
                        onClick={() => handleToggleAutostart(service.name)}
                        className={`p-2 rounded-lg transition-colors ${isAutostart(service.name)
                          ? 'bg-blue-500/10 text-blue-500 hover:bg-blue-500/20'
                          : 'bg-surface-inset hover:bg-hover text-content-secondary hover:text-content'
                          }`}
                        title={isAutostart(service.name) ? 'Disable autostart' : 'Autostart on launch'}
                      >
                        <Zap size={16} />
                      </button>
                    )}

                    {/* Reload button (for services that support it, when running) */}
                    {supportsReload(service.service_type) && service.status === 'running' && (
                      <button
                        onClick={() => handleReloadService(service.name)}
                        disabled={reloading === service.name}
                        className="p-2 rounded-lg bg-blue-500/10 hover:bg-blue-500/20 text-blue-500 transition-colors disabled:opacity-50"
                        title="Reload configuration"
                      >
                        {reloading === service.name ? (
                          <Loader2 size={16} className="animate-spin" />
                        ) : (
                          <RotateCw size={16} />
                        )}
                      </button>
                    )}

                    {/* Start/Stop button — only for daemons */}
                    {isStartable(service.service_type) && (
                      <button
                        onClick={() => handleToggleService(service.name, service.status)}
                        disabled={processing === service.name || service.status === 'starting' || service.status === 'stopping'}
                        className={`p-2 rounded-lg transition-colors disabled:opacity-50 ${service.status === 'running'
                          ? 'bg-red-500/10 hover:bg-red-500/20 text-red-500'
                          : 'bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-500'
                          }`}
                        title={service.status === 'running' ? 'Stop' : 'Start'}
                      >
                        {processing === service.name ? (
                          <Loader2 size={16} className="animate-spin" />
                        ) : service.status === 'running' ? (
                          <Square size={16} />
                        ) : (
                          <Play size={16} />
                        )}
                      </button>
                    )}

                    {/* Uninstall button */}
                    <button
                      onClick={() => handleUninstall(service.name, service.service_type, service.path)}
                      disabled={processing === service.name || service.status === 'running'}
                      className="p-2 rounded-lg hover:bg-red-500/20 hover:text-red-500 text-content-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      title={service.status === 'running' ? 'Stop service before uninstalling' : 'Uninstall'}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {subTab === 'install' && (
          <div className="space-y-8">
            {loading && (
              <div className="flex justify-center py-8">
                <Loader2 className="animate-spin text-emerald-500" size={32} />
              </div>
            )}

            {!loading && (
              <>
                {SERVICE_CATALOG.filter(s => s.group === 'server').map(svc => (
                  <ServiceGroup
                    key={svc.key}
                    title={svc.title}
                    icon={svc.icon}
                    versions={available[svc.key] || []}
                    type={svc.key}
                    onInstall={handleInstall}
                    processing={processing}
                    isInstalled={(v) => isVersionInstalled(svc.key, v)}
                    installedServices={services}
                  />
                ))}

                {SERVICE_CATALOG.some(s => s.group === 'devtools') && (
                  <div className="border-t border-edge pt-6 mt-2">
                    <h3 className="text-lg font-semibold mb-4 text-content-secondary">Developer Tools</h3>
                  </div>
                )}

                {SERVICE_CATALOG.filter(s => s.group === 'devtools').map(svc => (
                  <ServiceGroup
                    key={svc.key}
                    title={svc.title}
                    icon={svc.icon}
                    versions={available[svc.key] || []}
                    type={svc.key}
                    onInstall={handleInstall}
                    processing={processing}
                    isInstalled={(v) => isVersionInstalled(svc.key, v)}
                    installedServices={services}
                  />
                ))}
              </>
            )}
          </div>
        )}

        {subTab === 'tools' && (
          <div className="space-y-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <ComposerManager />
              <MailManager />
              <MeilisearchManager />
              <CliManager />
            </div>
            <CliCommandReference />
          </div>
        )}

        {subTab === 'logs' && (
          <LogViewer />
        )}
      </div>

      {/* Service Config Drawer */}
      <ServiceConfigDrawer
        isOpen={configDrawer.isOpen}
        onClose={closeConfigDrawer}
        serviceName={configDrawer.serviceName}
        serviceType={configDrawer.serviceType}
        serviceVersion={configDrawer.serviceVersion}
      />
    </div>
  );
}

function ServiceGroup({
  title,
  icon,
  versions,
  type,
  onInstall,
  processing,
  isInstalled,
  installedServices
}: {
  title: string;
  icon: string;
  versions: ServiceVersion[];
  type: string;
  onInstall: (type: string, version: ServiceVersion) => void;
  processing: string | null;
  isInstalled: (version: string) => boolean;
  installedServices?: { service_type: string; version: string }[];
}) {
  const source = versions[0]?.source;
  const sourceLabel = source === 'Cache' ? 'cached' : source === 'Fallback' ? 'fallback' : 'live';
  const sourceColor =
    source === 'Fallback' ? 'text-amber-500' :
      source === 'Cache' ? 'text-blue-400' :
        'text-emerald-500';

  if (versions.length === 0) {
    // Check if this service type is already installed even without registry versions
    const installedInfo = installedServices?.find(s => s.service_type === type);
    return (
      <div>
        <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
          <span className="text-xl">{icon}</span>
          {title}
          {installedInfo && (
            <span className="text-xs font-normal text-emerald-500 bg-emerald-500/10 px-2 py-0.5 rounded-full flex items-center gap-1">
              <CheckCircle size={12} /> v{installedInfo.version}
            </span>
          )}
        </h3>
        {!installedInfo && (
          <p className="text-content-muted text-sm">No versions available in registry</p>
        )}
      </div>
    );
  }

  // Find any installed version of this service so we can surface it even
  // when isVersionInstalled() can't match against registry version strings
  // (e.g. registry has "3.13.5" but local exe reports "3.13.6").
  const installedSummary = installedServices?.find((s) => s.service_type === type);

  return (
    <div>
      <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
        <span className="text-xl">{icon}</span>
        {title}
        {installedSummary && (
          <span className="text-xs font-normal text-emerald-500 bg-emerald-500/10 px-2 py-0.5 rounded-full flex items-center gap-1">
            <CheckCircle size={12} /> v{installedSummary.version} installed
          </span>
        )}
        <span className="text-xs font-normal text-content-muted bg-surface-raised px-2 py-0.5 rounded-full">
          {versions.length} versions
        </span>
        <span className={`text-xs font-normal ${sourceColor} bg-surface-raised px-2 py-0.5 rounded-full`}>
          {sourceLabel}
        </span>
      </h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3">
        {versions.map((v) => {
          const installed = isInstalled(v.version);
          const isProcessing = processing === `${type}-${v.version}`;

          return (
            <div
              key={v.version}
              className={`bg-surface border p-3 rounded-lg flex justify-between items-center transition-colors ${installed
                ? 'border-emerald-500/30 bg-emerald-500/5'
                : 'border-edge hover:border-edge'
                }`}
            >
              <div>
                <div className="font-medium flex items-center gap-2">
                  v{v.version}
                  {installed && (
                    <CheckCircle size={14} className="text-emerald-500" />
                  )}
                </div>
                <div className="text-xs text-content-muted truncate max-w-[150px]">
                  {v.filename}
                </div>
              </div>
              {installed ? (
                <span className="px-3 py-1.5 bg-emerald-500/10 text-emerald-500 text-xs rounded-md font-medium">
                  Installed
                </span>
              ) : (
                <button
                  onClick={() => onInstall(type, v)}
                  disabled={isProcessing}
                  className={`p-2 rounded-md transition-colors ${isProcessing
                    ? 'bg-surface-raised text-content-secondary'
                    : 'bg-surface-raised hover:bg-emerald-600 hover:text-content text-content-secondary cursor-pointer'
                    }`}
                  title={`Download ${type} ${v.version}`}
                >
                  {isProcessing ? (
                    <Loader2 size={16} className="animate-spin" />
                  ) : (
                    <Download size={16} />
                  )}
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ─── VersionPicker ─────────────────────────────────────────────────────
// Dropdown attached to each ServiceCard. Lists all installed versions for
// the service, marks the active one, and lets the user switch or remove a
// version. Versioned services only — PHP gets its own per-version cards
// (handled elsewhere) so the picker is hidden for PHP.

function VersionPicker({
  serviceType,
  serviceName,
  currentVersion,
  serviceStatus,
  onChanged,
  disableSwitch = false,
  versionStatuses,
  onToggleVersion,
}: {
  serviceType: string;
  /// Service name as known to ServiceManager (== service.name). Used to
  /// auto-restart through startServiceByName after a successful switch.
  /// Optional because PHP-mode picker doesn't switch.
  serviceName?: string;
  currentVersion: string;
  serviceStatus: string;
  onChanged: () => void | Promise<void>;
  /// PHP-style services run all versions concurrently — no "active" version
  /// to switch to. With `disableSwitch=true` the picker becomes info-only:
  /// lists installed versions + lets the user remove a single version, but
  /// clicking a row does nothing.
  disableSwitch?: boolean;
  /// Per-version runtime info, keyed by the directory name returned from
  /// list_service_versions ("8.4"). When provided, each row shows a status
  /// dot and (with `onToggleVersion`) a Start/Stop button.
  versionStatuses?: Record<string, { name: string; status: string; port?: number | null }>;
  /// Callback invoked when the per-row Start/Stop button is clicked. Only
  /// used when `versionStatuses` is provided.
  onToggleVersion?: (
    dirVersion: string,
    serviceName: string,
    currentStatus: string,
  ) => Promise<void> | void;
}) {
  const { addToast, startServiceByName } = useApp();
  const [open, setOpen] = useState(false);
  const [installed, setInstalled] = useState<string[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  const refresh = async () => {
    setLoading(true);
    try {
      const list = await listServiceVersions(serviceType);
      setInstalled(list.installed);
      setActive(list.active);
    } catch (e) {
      console.error('Failed to list versions:', e);
    } finally {
      setLoading(false);
    }
  };

  // Lazy-load on first open
  useEffect(() => {
    if (open) refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  // Click-outside to close
  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (ref.current && target && !ref.current.contains(target)) setOpen(false);
    };
    window.addEventListener('mousedown', onClick);
    return () => window.removeEventListener('mousedown', onClick);
  }, [open]);

  const handleActivate = async (version: string) => {
    if (disableSwitch) return; // info-only mode (e.g. PHP)
    if (version === active) return;
    setBusy(version);
    try {
      // set_active_service_version kills the running binary first as part
      // of the junction repoint. If the service was running, we now bring
      // it back up automatically so the user doesn't have to remember.
      const wasRunning = serviceStatus === 'running';
      await setActiveServiceVersion(serviceType, version);

      let restartedOk = false;
      if (wasRunning && serviceName) {
        try {
          await startServiceByName(serviceName);
          restartedOk = true;
        } catch (restartErr) {
          console.error(`Auto-restart of ${serviceName} failed:`, restartErr);
        }
      }

      addToast({
        type: 'success',
        message: wasRunning
          ? restartedOk
            ? `Switched ${serviceType} to v${version} and restarted.`
            : `Switched ${serviceType} to v${version}. Auto-restart failed — start it manually.`
          : `Switched ${serviceType} to v${version}.`,
      });
      setActive(version);
      setOpen(false);
      await onChanged();
    } catch (e: any) {
      addToast({ type: 'error', message: `Switch failed: ${e}` });
    } finally {
      setBusy(null);
    }
  };

  const handleRemove = async (version: string, e: React.MouseEvent) => {
    e.stopPropagation();
    // Path layout differs: junction-based services live under .versions/,
    // PHP keeps the legacy `bin/php/<ver>/` direct layout.
    const path = disableSwitch
      ? `bin/${serviceType}/${version}/`
      : `bin/.versions/${serviceType}/${version}/`;
    if (
      !window.confirm(
        `Remove ${serviceType} v${version}? Files in ${path} will be deleted.`,
      )
    ) {
      return;
    }
    setBusy(version);
    try {
      await removeServiceVersion(serviceType, version);
      addToast({ type: 'success', message: `Removed ${serviceType} v${version}` });
      await refresh();
      await onChanged();
    } catch (err: any) {
      addToast({ type: 'error', message: `Remove failed: ${err}` });
    } finally {
      setBusy(null);
    }
  };

  // Hide picker entirely if there's only one version installed and it's the
  // current one — no choice to make. Still show on hover to confirm version.
  const hasMultiple = installed.length > 1;

  // PHP-style: every installed version is the "current" one for that card.
  // The button text and the per-row indicator should reflect "this is THE
  // version this card represents", not "switch active".
  const headerLabel = loading
    ? 'Loading…'
    : disableSwitch
      ? hasMultiple
        ? `${installed.length} PHP versions installed`
        : 'Only PHP version installed'
      : hasMultiple
        ? `${installed.length} versions installed`
        : 'Only one version installed';

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1 text-xs font-normal text-content-muted bg-surface-raised hover:bg-hover px-2 py-0.5 rounded transition-colors cursor-pointer"
        title={disableSwitch ? 'Show all installed PHP versions' : 'Switch version'}
      >
        v{currentVersion}
        <ChevronDown size={11} className={`transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-1 z-30 min-w-[220px] bg-surface-raised border border-edge rounded-lg shadow-xl overflow-hidden">
          <div className="px-3 py-2 text-[11px] uppercase tracking-wide text-content-muted border-b border-edge/50">
            {headerLabel}
          </div>
          {!loading && installed.length === 0 && (
            <div className="px-3 py-3 text-xs text-content-muted">
              Install another version from the Install tab.
            </div>
          )}
          {installed.map((v) => {
            // For switchable services the row's "selected" highlight tracks
            // which version is *active* (the junction target). For PHP
            // (info-only) we highlight rows that are currently running so
            // the user sees runtime state at a glance.
            const isActive = !disableSwitch && v === active;
            const isRunning = versionStatuses?.[v]?.status === 'running';
            const showCheck = disableSwitch ? isRunning : isActive;
            const highlight = disableSwitch ? isRunning : isActive;
            const isBusy = busy === v;
            const rowClickable = !disableSwitch && !isBusy;
            return (
              <div
                key={v}
                className={`group flex items-center justify-between px-3 py-2 text-sm transition-colors ${
                  highlight ? 'bg-emerald-500/5 text-emerald-400' : 'hover:bg-hover'
                } ${rowClickable ? 'cursor-pointer' : disableSwitch ? 'cursor-default' : ''}`}
                onClick={() => rowClickable && handleActivate(v)}
              >
                <div className="flex items-center gap-2">
                  {showCheck ? (
                    <Check size={12} className="text-emerald-500" />
                  ) : (
                    <span className="w-3" />
                  )}
                  {/* Per-version runtime indicator (PHP). Switchable services
                      have a single card-level status, so we skip the dot. */}
                  {versionStatuses && (
                    <span
                      className={`w-2 h-2 rounded-full ${
                        versionStatuses[v]?.status === 'running'
                          ? 'bg-emerald-500'
                          : 'bg-content-muted/40'
                      }`}
                      title={versionStatuses[v]?.status ?? 'stopped'}
                    />
                  )}
                  <span className="font-mono">v{v}</span>
                  {versionStatuses?.[v]?.port && (
                    <span className="text-[10px] text-content-muted font-mono">
                      :{versionStatuses[v]!.port}
                    </span>
                  )}
                  {isActive && (
                    <span className="text-[10px] text-emerald-500/80">active</span>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  {isBusy && <Loader2 size={12} className="animate-spin" />}
                  {/* Per-row Start/Stop (PHP only — versionStatuses+toggle). */}
                  {versionStatuses && onToggleVersion && versionStatuses[v] && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        const info = versionStatuses[v]!;
                        onToggleVersion(v, info.name, info.status);
                      }}
                      disabled={isBusy}
                      className={`p-1 rounded transition-colors ${
                        versionStatuses[v]!.status === 'running'
                          ? 'hover:bg-red-500/10 hover:text-red-400'
                          : 'hover:bg-emerald-500/10 hover:text-emerald-400'
                      }`}
                      title={
                        versionStatuses[v]!.status === 'running'
                          ? `Stop ${versionStatuses[v]!.name}`
                          : `Start ${versionStatuses[v]!.name}`
                      }
                    >
                      {versionStatuses[v]!.status === 'running' ? (
                        <Square size={11} />
                      ) : (
                        <Play size={11} />
                      )}
                    </button>
                  )}
                  <button
                    onClick={(e) => handleRemove(v, e)}
                    disabled={isBusy}
                    className="p-1 opacity-0 group-hover:opacity-100 hover:bg-red-500/10 hover:text-red-400 rounded transition-all"
                    title={`Remove v${v}`}
                  >
                    <Trash2 size={11} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/// Service types that use the multi-version layout. Mirror of the Rust list
/// in `services/version_manager.rs::VERSIONED_SERVICES`. PHP intentionally
/// excluded — it has its own per-version cards.
const MULTI_VERSION_SERVICES = new Set([
  'nginx', 'apache', 'mariadb', 'postgresql', 'mongodb', 'redis',
  'nodejs', 'python', 'bun', 'go', 'deno', 'mailpit', 'meilisearch',
]);
