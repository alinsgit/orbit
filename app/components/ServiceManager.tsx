import { useState, useEffect } from 'react';
import { Download, Loader2, Trash2, RefreshCw, Play, Square, CheckCircle, Terminal, CheckCircle2, RotateCw, Settings2 } from 'lucide-react';
import { getAvailableVersions, downloadService, uninstallService, refreshAllVersions, ServiceVersion, addServiceToPath, removeServiceFromPath, checkServicePathStatus, ServicePathStatus, reloadService } from '../lib/api';
import { useApp } from '../lib/AppContext';
import { ServiceConfigDrawer } from './ServiceConfigDrawer';
import { ServiceOverview } from './ServiceOverview';
import { ComposerManager } from './ComposerManager';
import { MailManager } from './MailManager';
import { getServiceIcon } from '../lib/serviceIcons';

export function ServiceManager() {
  const {
    services,
    refreshServices,
    startServiceByName,
    stopServiceByName,
    addToast
  } = useApp();

  const [subTab, setSubTab] = useState<'overview' | 'manage' | 'install' | 'tools'>('overview');
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

  // Check if a version is already installed
  const isVersionInstalled = (type: string, version: string) => {
    return services.some(s => {
      if (s.service_type !== type) return false;
      const installed = s.version.trim();
      const available = version.trim();

      // Exact match
      if (installed === available) return true;

      const iParts = installed.split('.');
      const aParts = available.split('.');

      // Multi-version: match major.minor (php 8.4.x, mariadb 11.4.x, python 3.13.x, nginx 1.28.x, go 1.22.x)
      if (['php', 'mariadb', 'postgresql', 'mongodb', 'python', 'nginx', 'go'].includes(type)) {
        return iParts[0] === aParts[0] && iParts[1] === aParts[1];
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
      await fetchAvailable(true);
      addToast({ type: 'success', message: 'Versions refreshed' });
    } catch (e) {
      console.error(e);
      addToast({ type: 'error', message: 'Failed to refresh versions' });
    } finally {
      setRefreshing(false);
    }
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
  }, []);

  // Check PATH statuses when services change (debounced)
  useEffect(() => {
    if (services.length > 0) {
      const timer = setTimeout(() => {
        checkAllPathStatuses();
      }, 500);
      return () => clearTimeout(timer);
    }
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

      await downloadService(version.download_url, filename, typeParam);
      await refreshServices();
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
    if (!confirm(`Are you sure you want to uninstall ${name}? This will remove all files.`)) {
      return;
    }

    setProcessing(name);
    try {
      await uninstallService(name, serviceType, path);
      await refreshServices();
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

  return (
    <div className="p-6 h-full flex flex-col">
      <header className="flex justify-between items-center mb-6">
        <div>
          <h2 className="text-2xl font-bold">Services</h2>
          <p className="text-content-secondary">Manage your local server components</p>
        </div>
        <div className="flex items-center gap-3">
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
          <div className="flex bg-surface-raised p-1 rounded-lg">
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
          </div>
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
            {services.map((service) => (
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
                        <span className="text-xs font-normal text-content-muted bg-surface-raised px-2 py-0.5 rounded">
                          v{service.version}
                        </span>
                      </h3>
                      <p className="text-sm text-content-secondary truncate max-w-[300px]">
                        {service.path}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {/* Status indicator */}
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

                    {/* Start/Stop button */}
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
                  />
                ))}
              </>
            )}
          </div>
        )}

        {subTab === 'tools' && (
          <div className="space-y-6">
            <ComposerManager />
            <MailManager />
          </div>
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
  isInstalled
}: {
  title: string;
  icon: string;
  versions: ServiceVersion[];
  type: string;
  onInstall: (type: string, version: ServiceVersion) => void;
  processing: string | null;
  isInstalled: (version: string) => boolean;
}) {
  const source = versions[0]?.source;
  const sourceLabel = source === 'Cache' ? 'cached' : source === 'Fallback' ? 'fallback' : 'live';
  const sourceColor =
    source === 'Fallback' ? 'text-amber-500' :
      source === 'Cache' ? 'text-blue-400' :
        'text-emerald-500';

  if (versions.length === 0) {
    return (
      <div>
        <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
          <span className="text-xl">{icon}</span>
          {title}
        </h3>
        <p className="text-content-muted text-sm">No versions available</p>
      </div>
    );
  }

  return (
    <div>
      <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
        <span className="text-xl">{icon}</span>
        {title}
        <span className="text-xs font-normal text-content-muted bg-surface-raised px-2 py-0.5 rounded-full">
          {versions.length} versions
        </span>
        <span className={`text-xs font-normal ${sourceColor} bg-surface-raised px-2 py-0.5 rounded-full`}>
          {sourceLabel}
        </span>
      </h3>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
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
                  disabled={processing !== null}
                  className={`p-2 rounded-md transition-colors ${isProcessing
                      ? 'bg-surface-raised text-content-secondary'
                      : 'bg-surface-raised hover:bg-emerald-600 hover:text-content text-content-secondary'
                    }`}
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
