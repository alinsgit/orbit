import { useState, useEffect } from 'react';
import {
  FolderOpen, Plus, Trash2, Globe, ExternalLink, Loader2, Shield,
  AlertTriangle, RefreshCw, CheckCircle2, XCircle, Settings2,
  FileCode, Layers, Database, ShoppingCart, Sparkles, CheckCircle, FileDown, FileUp,
  Play, Square
} from 'lucide-react';
import {
  getSites, createSite, deleteSite, updateSite, regenerateSiteConfig,
  generateSslCert, nginxTestConfig, nginxReload, nginxStatus,
  addHostElevated, SiteWithStatus, Site, WebServer, reloadService,
  getSslStatus, getWorkspacePath, startTunnel, stopTunnel, getTunnelUrl,
  installMkcert, installSslCa, exportSites, importSites, SiteExport, SslStatus,
  scaffoldBasicProject, startSiteApp, stopSiteApp, getSiteAppStatus
} from '../lib/api';
import { useApp } from '../lib/AppContext';
import { open } from '@tauri-apps/plugin-dialog';
import { open as shellOpen } from '@tauri-apps/plugin-shell';
import { load } from '@tauri-apps/plugin-store';

type SiteTemplate = 'http' | 'laravel' | 'wordpress' | 'litecart' | 'static' | 'nextjs' | 'astro' | 'nuxt' | 'vue' | 'django' | 'sveltekit' | 'remix';

const WEB_SERVER_INFO: Record<WebServer, { label: string; icon: string; color: string }> = {
  nginx: { label: 'Nginx', icon: '🌐', color: 'bg-green-500/10 text-green-400' },
  apache: { label: 'Apache', icon: '🪶', color: 'bg-orange-500/10 text-orange-400' }
};

const PHP_TEMPLATES: SiteTemplate[] = ['http', 'laravel', 'wordpress', 'litecart'];

const TEMPLATE_INFO: Record<SiteTemplate, { label: string; icon: React.ReactNode; description: string }> = {
  http: {
    label: 'Standard PHP',
    icon: <FileCode size={14} />,
    description: 'Basic PHP application with index.php'
  },
  laravel: {
    label: 'Laravel',
    icon: <Layers size={14} />,
    description: 'Laravel/Symfony with public directory'
  },
  wordpress: {
    label: 'WordPress',
    icon: <Database size={14} />,
    description: 'WordPress with optimized rules'
  },
  litecart: {
    label: 'LiteCart',
    icon: <ShoppingCart size={14} />,
    description: 'LiteCart e-commerce platform'
  },
  static: {
    label: 'Static',
    icon: <Globe size={14} />,
    description: 'Static HTML/CSS/JS files only'
  },
  nextjs: {
    label: 'Next.js',
    icon: <Layers size={14} />,
    description: 'React Framework'
  },
  astro: {
    label: 'Astro',
    icon: <Globe size={14} />,
    description: 'Astro Framework'
  },
  nuxt: {
    label: 'Nuxt',
    icon: <Layers size={14} />,
    description: 'Vue Framework'
  },
  vue: {
    label: 'Vue',
    icon: <Layers size={14} />,
    description: 'Vue.js Application'
  },
  django: {
    label: 'Django',
    icon: <Layers size={14} />,
    description: 'Python Django application'
  },
  sveltekit: {
    label: 'SvelteKit',
    icon: <Layers size={14} />,
    description: 'SvelteKit application'
  },
  remix: {
    label: 'Remix',
    icon: <Layers size={14} />,
    description: 'Remix React framework'
  }
};

export function SitesManager() {
  const { getInstalledPhpVersions, services, addToast, openTerminalForSite } = useApp();

  // Check which web servers are installed
  const nginxInstalled = services.some(s => s.service_type === 'nginx');
  const apacheInstalled = services.some(s => s.service_type === 'apache');
  const availableWebServers: WebServer[] = [
    ...(nginxInstalled ? ['nginx' as WebServer] : []),
    ...(apacheInstalled ? ['apache' as WebServer] : [])
  ];
  const defaultWebServer: WebServer = nginxInstalled ? 'nginx' : (apacheInstalled ? 'apache' : 'nginx');

  const [sites, setSites] = useState<SiteWithStatus[]>([]);
  const [showAddForm, setShowAddForm] = useState(false);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  // Nginx status
  const [isNginxRunning, setIsNginxRunning] = useState(false);
  const [reloadingNginx, setReloadingNginx] = useState(false);
  const [nginxMessage, setNginxMessage] = useState<{ type: 'success' | 'error'; text: string; domain?: string } | null>(null);
  const [fixingHosts, setFixingHosts] = useState(false);

  // Get PHP versions from global context
  const phpVersions = getInstalledPhpVersions();

  // New site form
  const [newSite, setNewSite] = useState<Site>({
    domain: '',
    path: '',
    port: 80,
    php_version: '',
    ssl_enabled: false,
    template: 'http',
    web_server: defaultWebServer
  });

  const [processing, setProcessing] = useState<string | null>(null);
  const [editingSite, setEditingSite] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<Site | null>(null);

  // SSL status for mkcert/CA check
  const [sslReady, setSslReady] = useState(false);
  const [showSslDropdown, setShowSslDropdown] = useState(false);
  const [sslStatus, setSslStatus] = useState<SslStatus | null>(null);
  const [sslLoading, setSslLoading] = useState(false);

  // Export/Import State
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);

  // Site App Process State
  const [appStatus, setAppStatus] = useState<Record<string, string>>({});
  const [appProcessing, setAppProcessing] = useState<string | null>(null);

  // Tunnel State
  const [activeTunnel, setActiveTunnel] = useState<{ domain: string; url: string } | null>(null);
  const [tunnelingDomain, setTunnelingDomain] = useState<string | null>(null);

  // Check SSL status
  const loadSslStatus = async () => {
    try {
      const status = await getSslStatus();
      setSslStatus(status);
      setSslReady(status.mkcert_installed && status.ca_installed);
    } catch (e) {
      console.error('Failed to load SSL status:', e);
    }
  };

  useEffect(() => {
    loadSslStatus();
  }, []);

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
          addToast({ type: 'warning', message: `Imported ${result.imported} sites, skipped ${result.skipped}. Errors: ${result.errors.length}` });
        } else {
          const skipMsg = result.skipped > 0 ? `, skipped ${result.skipped} existing` : '';
          addToast({ type: 'success', message: `Successfully imported ${result.imported} sites${skipMsg}` });
        }
        await refreshSites();
      } catch (e: any) {
        addToast({ type: 'error', message: `Failed to import sites: ${e.message || e}` });
      } finally {
        setImporting(false);
      }
    };
    input.click();
  };

  // Load sites and check nginx status
  const refreshSites = async () => {
    setRefreshing(true);
    try {
      const [s, running] = await Promise.all([
        getSites(),
        nginxStatus()
      ]);
      setSites(s);
      setIsNginxRunning(running);
    } catch (e) {
      console.error('Failed to refresh sites:', e);
    } finally {
      setRefreshing(false);
    }
  };

  // Refresh app status for sites with dev_command
  const refreshAppStatus = async (siteList: SiteWithStatus[]) => {
    const withCommand = siteList.filter(s => s.dev_command);
    if (withCommand.length === 0) return;
    const statuses: Record<string, string> = {};
    await Promise.all(
      withCommand.map(async (s) => {
        try {
          statuses[s.domain] = await getSiteAppStatus(s.domain);
        } catch {
          statuses[s.domain] = 'stopped';
        }
      })
    );
    setAppStatus(statuses);
  };

  useEffect(() => {
    refreshSites().then(() => {
      // App status will be refreshed after sites load
    });
  }, []);

  // Poll app status when sites change
  useEffect(() => {
    if (sites.length > 0) {
      refreshAppStatus(sites);
    }
  }, [sites.length]);

  const handleStartApp = async (domain: string) => {
    setAppProcessing(domain);
    try {
      await startSiteApp(domain);
      setAppStatus(prev => ({ ...prev, [domain]: 'running' }));
      addToast({ type: 'success', message: `App started for ${domain}` });
    } catch (e: any) {
      addToast({ type: 'error', message: e?.toString() || 'Failed to start app' });
    } finally {
      setAppProcessing(null);
    }
  };

  const handleStopApp = async (domain: string) => {
    setAppProcessing(domain);
    try {
      await stopSiteApp(domain);
      setAppStatus(prev => ({ ...prev, [domain]: 'stopped' }));
      addToast({ type: 'success', message: `App stopped for ${domain}` });
    } catch (e: any) {
      addToast({ type: 'error', message: e?.toString() || 'Failed to stop app' });
    } finally {
      setAppProcessing(null);
    }
  };

  // Handle nginx reload
  const handleNginxReload = async () => {
    setReloadingNginx(true);
    setNginxMessage(null);
    try {
      // First test config
      await nginxTestConfig();
      // Then reload
      await nginxReload();
      setNginxMessage({ type: 'success', text: 'Nginx reloaded successfully' });
      setIsNginxRunning(true);
    } catch (e: any) {
      setNginxMessage({ type: 'error', text: e.toString() });
    } finally {
      setReloadingNginx(false);
      setTimeout(() => setNginxMessage(null), 5000);
    }
  };

  // Fix hosts file with admin elevation
  const handleFixHosts = async (domain: string) => {
    setFixingHosts(true);
    try {
      await addHostElevated(domain);
      setNginxMessage({ type: 'success', text: `Domain ${domain} added to hosts file successfully!` });
      setTimeout(() => setNginxMessage(null), 5000);
    } catch (e: any) {
      setNginxMessage({ type: 'error', text: `Failed to add domain: ${e}` });
    } finally {
      setFixingHosts(false);
    }
  };

  // Detect template from path
  const detectTemplate = (path: string): SiteTemplate => {
    if (!path) return 'http';
    const pathLower = path.toLowerCase();
    if (pathLower.includes('laravel') || pathLower.includes('symfony')) return 'laravel';
    if (pathLower.includes('wordpress') || pathLower.includes('wp-')) return 'wordpress';
    if (pathLower.includes('litecart')) return 'litecart';
    return 'http';
  };

  // Handle path selection
  const selectFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Project Directory'
    });
    if (typeof selected === 'string') {
      const detectedTemplate = detectTemplate(selected);
      setNewSite({
        ...newSite,
        path: selected,
        template: detectedTemplate
      });
    }
  };

  // Handle Scaffold
  const handleScaffold = async () => {
    if (!newSite.domain) {
      addToast({ type: 'warning', message: 'Enter a domain to use as the project name' });
      return;
    }

    try {
      const wp = await getWorkspacePath();
      if (!wp) {
        addToast({ type: 'error', message: 'No Workspace Directory set. Please set it in Settings.' });
        return;
      }

      const projectName = newSite.domain.split('.')[0];
      const template = newSite.template || '';
      const separator = navigator.userAgent.includes('Win') ? '\\' : '/';
      const projectPath = `${wp}${separator}${projectName}`;

      // Basic templates: create files directly via backend
      if (template === 'http' || template === 'static' || template === 'litecart') {
        try {
          const result = await scaffoldBasicProject(projectPath, template);
          addToast({ type: 'success', message: result });
          setNewSite({ ...newSite, path: projectPath });
        } catch (e: any) {
          addToast({ type: 'error', message: `Scaffolding failed: ${e}` });
        }
        return;
      }

      // Terminal-based scaffolding for frameworks
      let scaffoldCmd = '';
      if (template === 'nextjs') {
        scaffoldCmd = `npx --yes create-next-app@latest ${projectName} --typescript --tailwind --eslint --app --src-dir --import-alias "@/*" --use-npm`;
      } else if (template === 'nuxt') {
        scaffoldCmd = `npx --yes nuxi@latest init ${projectName}`;
      } else if (template === 'vue') {
        scaffoldCmd = `npx --yes create-vue@latest ${projectName} --yes`;
      } else if (template === 'astro') {
        scaffoldCmd = `npx --yes create-astro@latest ${projectName} --yes`;
      } else if (template === 'laravel') {
        scaffoldCmd = `composer create-project laravel/laravel ${projectName}`;
      } else if (template === 'wordpress') {
        scaffoldCmd = `php -r "copy('https://wordpress.org/latest.zip', 'wp.zip'); $z = new ZipArchive; if ($z->open('wp.zip') === TRUE) { $z->extractTo('.'); $z->close(); rename('wordpress', '${projectName}'); unlink('wp.zip'); }"`;
      } else {
        addToast({ type: 'warning', message: 'Unsupported template configuration.'});
        return;
      }

      // Open terminal for site and run scaffold command
      openTerminalForSite(newSite.domain || 'scaffold', wp, scaffoldCmd);
      addToast({ type: 'info', message: 'Executing scaffold in the Integrated Terminal...' });

      // Auto-fill path — Laravel needs /public as nginx root
      setNewSite({ ...newSite, path: projectPath });

    } catch (e: any) {
      addToast({ type: 'error', message: `Scaffolding failed: ${e}` });
    }
  };

  // Handle add site
  const handleAddSite = async () => {
    if (!newSite.domain || !newSite.path) {
      addToast({ type: 'warning', message: 'Domain and path are required' });
      return;
    }

    setLoading(true);
    try {
      // SSL certificate generation if enabled
      if (newSite.ssl_enabled) {
        await generateSslCert(newSite.domain);
      }

      const result = await createSite(newSite);

      // Reload the appropriate web server to apply changes
      try {
        if (newSite.web_server === 'apache') {
          await reloadService('apache');
        } else {
          await nginxTestConfig();
          await nginxReload();
        }
      } catch (reloadError) {
        console.warn('Web server reload failed:', reloadError);
      }

      // Show warning if hosts file couldn't be updated
      if (result.warning) {
        setNginxMessage({ type: 'error', text: result.warning, domain: result.domain });
      }

      // Open terminal in the new site's directory
      const sitePath = newSite.path;

      setNewSite({
        domain: '',
        path: '',
        port: 80,
        php_version: '',
        ssl_enabled: false,
        template: 'http',
        web_server: defaultWebServer
      });
      setShowAddForm(false);
      await refreshSites();

      // Open terminal in the new site's directory
      if (sitePath) {
        openTerminalForSite(result.domain, sitePath);
      }
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: 'Failed to create site: ' + e });
    } finally {
      setLoading(false);
    }
  };

  // Handle delete site
  const handleDeleteSite = async (domain: string, webServer?: WebServer) => {
    setProcessing(domain);
    try {
      await deleteSite(domain);
      // Reload appropriate web server
      if (webServer === 'apache') {
        await reloadService('apache');
      } else {
        await nginxReload();
      }
      await refreshSites();
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: 'Failed to delete site: ' + e });
    } finally {
      setProcessing(null);
    }
  };

  // Handle edit site
  const startEditing = (site: SiteWithStatus) => {
    setEditingSite(site.domain);
    setEditForm({
      domain: site.domain,
      path: site.path,
      port: site.port,
      php_version: site.php_version,
      php_port: site.php_port,
      ssl_enabled: site.ssl_enabled,
      template: site.template || 'http',
      web_server: site.web_server || 'nginx'
    });
  };

  const handleUpdateSite = async () => {
    if (!editingSite || !editForm) return;

    setProcessing(editingSite);
    try {
      // Generate SSL certificate if SSL was just enabled
      const existingSite = sites.find(s => s.domain === editingSite);
      if (editForm.ssl_enabled && existingSite && !existingSite.ssl_enabled) {
        await generateSslCert(editForm.domain);
      }

      await updateSite(editingSite, editForm);

      // Reload appropriate web server
      if (editForm.web_server === 'apache') {
        await reloadService('apache');
      } else {
        await nginxReload();
      }

      await refreshSites();
      setEditingSite(null);
      setEditForm(null);
      addToast({ type: 'success', message: `Site ${editForm.domain} updated successfully` });
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: 'Failed to update site: ' + e });
    } finally {
      setProcessing(null);
    }
  };

  // Handle regenerate config
  const handleRegenerateConfig = async (domain: string, webServer?: WebServer) => {
    setProcessing(domain);
    try {
      await regenerateSiteConfig(domain);
      // Reload appropriate web server
      if (webServer === 'apache') {
        await reloadService('apache');
      } else {
        await nginxReload();
      }
      addToast({ type: 'success', message: 'Config regenerated successfully!' });
      await refreshSites();
    } catch (e: any) {
      console.error(e);
      addToast({ type: 'error', message: `Failed to regenerate config: ${e}` });
    } finally {
      setProcessing(null);
    }
  };

  const getNgrokToken = async (): Promise<string | null> => {
    try {
      const store = await load('.settings.json', { autoSave: false, defaults: { workspacePath: '', ngrokAuthToken: '' } });
      return await store.get<string>('ngrokAuthToken') || null;
    } catch {
      return null;
    }
  };

  const handleStartTunnel = async (site: Site) => {
    setTunnelingDomain(site.domain);
    try {
      const token = await getNgrokToken();
      if (!token) {
        addToast({ type: 'error', message: 'Ngrok Auth Token missing. Please save it in Settings first.' });
        return;
      }
      
      const portToTunnel = site.ssl_enabled ? (site.port !== 80 && site.port !== 443 ? site.port : 443) : (site.port !== 80 && site.port !== 443 ? site.port : 80);
      
      const res = await startTunnel(site.domain, portToTunnel, token);
      if (res.success) {
        // Poll for public URL
        let url = null;
        for (let i = 0; i < 5; i++) {
          try {
            url = await getTunnelUrl();
            if (url) break;
          } catch(e) {
            // Ngrok node might still be waking up, wait 1 sec
            await new Promise(r => setTimeout(r, 1000));
          }
        }
        
        if (url) {
          setActiveTunnel({ domain: site.domain, url });
          addToast({ type: 'success', message: 'Tunnel is live!' });
        } else {
          addToast({ type: 'error', message: 'Tunnel started but could not resolve public URL.' });
        }
      }
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to start tunnel: ${e}` });
    } finally {
      setTunnelingDomain(null);
    }
  };

  const handleStopTunnel = async () => {
    setTunnelingDomain('stopping');
    try {
      await stopTunnel();
      setActiveTunnel(null);
      addToast({ type: 'success', message: 'Tunnel stopped.' });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to stop tunnel: ${e}` });
    } finally {
      setTunnelingDomain(null);
    }
  };

  return (
    <div className="p-6 h-full flex flex-col">
      {/* Header */}
      <header className="flex justify-between items-center mb-6">
        <div>
          <h2 className="text-2xl font-bold">Sites</h2>
          <p className="text-content-secondary">Manage your local development sites</p>
        </div>
        <div className="flex items-center gap-3">
          {/* SSL Tools */}
          <div className="relative">
            <button
              onClick={() => setShowSslDropdown(!showSslDropdown)}
              className="p-2 bg-surface-raised hover:bg-hover rounded-lg transition-colors cursor-pointer flex items-center gap-2 border border-edge"
              title="SSL & Root CA Settings"
            >
              <Shield size={16} className={sslReady ? "text-emerald-500" : "text-amber-500"} />
            </button>

            {showSslDropdown && (
              <div className="absolute top-full right-0 mt-2 w-72 bg-surface-raised border border-edge rounded-lg shadow-xl shadow-black/20 z-50 p-4">
                <div className="flex items-center justify-between mb-3">
                  <h4 className="font-medium text-sm">Local SSL Root CA</h4>
                  <button onClick={() => setShowSslDropdown(false)} className="text-content-muted hover:text-content">
                    <XCircle size={16} />
                  </button>
                </div>

                <div className="space-y-3">
                  <div className="flex items-center gap-3">
                    <div className={`w-2 h-2 rounded-full ${sslStatus?.mkcert_installed ? 'bg-emerald-500' : 'bg-red-500'}`} />
                    <div className="flex-1 text-xs">
                      <p className="font-medium">mkcert Core</p>
                      <p className="text-content-muted">{sslStatus?.mkcert_installed ? 'Installed' : 'Missing'}</p>
                    </div>
                    {!sslStatus?.mkcert_installed && (
                      <button onClick={handleInstallMkcert} disabled={sslLoading} className="px-2 py-1 bg-emerald-600 hover:bg-emerald-500 text-xs rounded transition-colors disabled:opacity-50">
                        {sslLoading ? <Loader2 size={12} className="animate-spin" /> : 'Install'}
                      </button>
                    )}
                  </div>

                  {sslStatus?.mkcert_installed && (
                    <div className="flex items-center gap-3">
                      <div className={`w-2 h-2 rounded-full ${sslStatus?.ca_installed ? 'bg-emerald-500' : 'bg-amber-500'}`} />
                      <div className="flex-1 text-xs">
                        <p className="font-medium">Browser CA Trust</p>
                        <p className="text-content-muted">{sslStatus?.ca_installed ? 'Trusted' : 'Untrusted'}</p>
                      </div>
                      {!sslStatus?.ca_installed && (
                        <button onClick={handleInstallCa} disabled={sslLoading} className="px-2 py-1 bg-amber-600 hover:bg-amber-500 text-xs rounded transition-colors disabled:opacity-50">
                          {sslLoading ? <Loader2 size={12} className="animate-spin" /> : 'Install'}
                        </button>
                      )}
                    </div>
                  )}

                  {sslReady && (
                    <p className="text-xs text-emerald-400 pt-1 flex items-center gap-1.5">
                      <CheckCircle size={12} /> Ready to generate HTTPS sites.
                    </p>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Export/Import Backup */}
          <div className="flex items-center bg-surface-raised rounded-lg border border-edge">
            <button
              onClick={handleImportSites}
              disabled={importing}
              className="p-2 hover:bg-hover rounded-l-lg transition-colors cursor-pointer disabled:opacity-50 border-r border-edge"
              title="Import Sites Config"
            >
              {importing ? <Loader2 size={16} className="animate-spin" /> : <FileUp size={16} />}
            </button>
            <button
              onClick={handleExportSites}
              disabled={exporting}
              className="p-2 hover:bg-hover rounded-r-lg transition-colors cursor-pointer disabled:opacity-50"
              title="Export Sites Config"
            >
              {exporting ? <Loader2 size={16} className="animate-spin" /> : <FileDown size={16} />}
            </button>
          </div>

          {/* Nginx Status & Reload */}
          <div className="flex items-center gap-2 px-3 py-2 bg-surface-raised rounded-lg border border-edge">
            <div className={`w-2 h-2 rounded-full ${isNginxRunning ? 'bg-emerald-500' : 'bg-red-500'}`} />
            <span className="text-sm text-content-secondary hidden lg:inline">Nginx</span>
            <button
              onClick={handleNginxReload}
              disabled={reloadingNginx}
              className="p-1 hover:bg-hover rounded transition-colors cursor-pointer disabled:opacity-50"
              title="Reload Nginx"
            >
              <RefreshCw size={14} className={reloadingNginx ? 'animate-spin' : ''} />
            </button>
          </div>

          {/* Refresh */}
          <button
            onClick={refreshSites}
            disabled={refreshing}
            className="p-2 bg-surface-raised hover:bg-hover rounded-lg transition-colors cursor-pointer disabled:opacity-50 border border-edge"
            title="Refresh Sites"
          >
            <RefreshCw size={16} className={refreshing ? 'animate-spin' : ''} />
          </button>

          {/* Add Site */}
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors cursor-pointer shadow-lg shadow-emerald-900/20"
          >
            <Plus size={16} />
            Add Site
          </button>
        </div>
      </header>

      {/* Nginx Message */}
      {nginxMessage && (
        <div className={`mb-4 p-3 rounded-lg flex items-center justify-between ${nginxMessage.type === 'success'
          ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-400'
          : 'bg-amber-500/10 border border-amber-500/30 text-amber-400'
          }`}>
          <div className="flex items-center gap-2">
            {nginxMessage.type === 'success' ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
            <span className="text-sm">{nginxMessage.text}</span>
          </div>
          {nginxMessage.domain && nginxMessage.type === 'error' && (
            <button
              onClick={() => handleFixHosts(nginxMessage.domain!)}
              disabled={fixingHosts}
              className="px-3 py-1 bg-amber-600 hover:bg-amber-500 text-white rounded text-xs font-medium transition-colors cursor-pointer disabled:opacity-50 flex items-center gap-1"
            >
              {fixingHosts ? <Loader2 size={12} className="animate-spin" /> : <Shield size={12} />}
              Fix with Admin
            </button>
          )}
        </div>
      )}

      {/* Add Form */}
      {showAddForm && (
        <div className="bg-surface-raised border border-edge rounded-xl p-5 mb-6">
          <h3 className="font-semibold mb-4 flex items-center gap-2">
            <Plus size={18} className="text-emerald-500" />
            Add New Site
          </h3>

          <div className="grid grid-cols-2 gap-4 mb-4">
            {/* Domain */}
            <div>
              <label className="block text-sm text-content-secondary mb-1">Domain</label>
              <input
                type="text"
                value={newSite.domain}
                onChange={(e) => setNewSite({ ...newSite, domain: e.target.value })}
                placeholder="mysite.test"
                className="w-full px-3 py-2 bg-surface border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500"
              />
            </div>

            {/* Port */}
            <div>
              <label className="block text-sm text-content-secondary mb-1">Port</label>
              <input
                type="number"
                value={newSite.port}
                onChange={(e) => setNewSite({ ...newSite, port: parseInt(e.target.value) || 80 })}
                className="w-full px-3 py-2 bg-surface border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500"
              />
            </div>
          </div>

          {/* Path */}
          <div className="mb-4">
            <label className="block text-sm text-content-secondary mb-1">Project Path</label>
            <div className="flex gap-2">
              <input
                type="text"
                value={newSite.path}
                onChange={(e) => setNewSite({ ...newSite, path: e.target.value })}
                placeholder="C:/projects/mysite"
                className="flex-1 px-3 py-2 bg-surface border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500"
              />
              <button
                onClick={selectFolder}
                className="px-3 py-2 bg-surface-inset hover:bg-hover rounded-lg transition-colors cursor-pointer"
                title="Select Existing Folder"
              >
                <FolderOpen size={16} />
              </button>
              <button
                onClick={handleScaffold}
                disabled={!newSite.domain}
                className="px-3 py-2 bg-emerald-600/20 text-emerald-500 hover:bg-emerald-600/30 rounded-lg transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                title="Scaffold new project in Workspace"
              >
                <Sparkles size={16} />
                Generate Default
              </button>
            </div>
          </div>

          {/* Template & PHP Version */}
          <div className="grid grid-cols-2 gap-4 mb-4">
            {/* Template */}
            <div>
              <label className="block text-sm text-content-secondary mb-1">Template</label>
              <div className="grid grid-cols-2 gap-2">
                {(Object.keys(TEMPLATE_INFO) as SiteTemplate[]).map((template) => (
                  <button
                    key={template}
                    onClick={() => setNewSite({
                      ...newSite,
                      template,
                      ...(!PHP_TEMPLATES.includes(template) ? { php_version: undefined } : {})
                    })}
                    className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors cursor-pointer ${newSite.template === template
                      ? 'bg-emerald-600 text-white'
                      : 'bg-surface-raised hover:bg-hover text-content-secondary'
                      }`}
                  >
                    {TEMPLATE_INFO[template].icon}
                    {TEMPLATE_INFO[template].label}
                  </button>
                ))}
              </div>
            </div>

            {/* PHP Version */}
            <div>
              <label className="block text-sm text-content-secondary mb-1">PHP Version</label>
              {!PHP_TEMPLATES.includes(newSite.template as SiteTemplate) ? (
                <p className="text-sm text-content-muted py-2">Not needed for this template</p>
              ) : phpVersions.length === 0 ? (
                <p className="text-sm text-amber-500 py-2">No PHP installed. Install from Services tab.</p>
              ) : (
                <select
                  value={newSite.php_version || ''}
                  onChange={(e) => setNewSite({ ...newSite, php_version: e.target.value || undefined })}
                  className="w-full px-3 py-2 bg-surface border border-edge rounded-lg text-sm focus:outline-none focus:border-emerald-500"
                >
                  <option value="">Select PHP version</option>
                  {phpVersions.map((v) => (
                    <option key={v} value={v}>PHP {v}</option>
                  ))}
                </select>
              )}
            </div>
          </div>

          {/* Web Server & SSL */}
          <div className="grid grid-cols-2 gap-4 mb-4">
            {/* Web Server Selector */}
            <div>
              <label className="block text-sm text-content-secondary mb-1">Web Server</label>
              {availableWebServers.length === 0 ? (
                <p className="text-sm text-amber-500 py-2">No web server installed. Install Nginx or Apache from Services tab.</p>
              ) : (
                <div className="flex gap-2">
                  {availableWebServers.map((ws) => (
                    <button
                      key={ws}
                      onClick={() => setNewSite({ ...newSite, web_server: ws })}
                      className={`flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors cursor-pointer ${newSite.web_server === ws
                        ? ws === 'nginx' ? 'bg-green-600 text-white' : 'bg-orange-600 text-white'
                        : 'bg-surface-raised hover:bg-hover text-content-secondary'
                        }`}
                    >
                      <span>{WEB_SERVER_INFO[ws].icon}</span>
                      {WEB_SERVER_INFO[ws].label}
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* SSL */}
            <div className="flex flex-col gap-1 pb-1">
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="ssl_enabled"
                  checked={newSite.ssl_enabled}
                  disabled={!sslReady}
                  onChange={(e) => setNewSite({ ...newSite, ssl_enabled: e.target.checked })}
                  className="w-4 h-4 rounded border-edge bg-surface text-emerald-500 focus:ring-emerald-500 focus:ring-offset-0 disabled:opacity-50"
                />
                <label htmlFor="ssl_enabled" className={`text-sm flex items-center gap-2 ${sslReady ? 'text-content-secondary' : 'text-content-muted'}`}>
                  <Shield size={14} className={sslReady ? 'text-emerald-500' : 'text-content-muted'} />
                  Enable SSL (HTTPS)
                </label>
              </div>
              {!sslReady && (
                <p className="text-xs text-amber-400 flex items-center gap-1">
                  <AlertTriangle size={12} />
                  Install mkcert and Root CA in Settings first
                </p>
              )}
            </div>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-2">
            <button
              onClick={() => setShowAddForm(false)}
              className="px-4 py-2 bg-surface-inset hover:bg-hover rounded-lg text-sm font-medium transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              onClick={handleAddSite}
              disabled={loading || !newSite.domain || !newSite.path}
              className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-surface-raised disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors cursor-pointer"
            >
              {loading ? (
                <>
                  <Loader2 size={16} className="animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <Plus size={16} />
                  Create Site
                </>
              )}
            </button>
          </div>
        </div>
      )}

      {/* Sites List */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {sites.length === 0 ? (
          <div className="text-center py-12 text-content-muted border-2 border-dashed border-edge rounded-xl">
            <Globe size={48} className="mx-auto mb-4 opacity-50" />
            <p>No sites configured yet.</p>
            <button
              onClick={() => setShowAddForm(true)}
              className="text-emerald-500 hover:underline mt-2 cursor-pointer"
            >
              Add your first site
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {sites.map((site) => (
              <div
                key={site.domain}
                className={`bg-surface-raised border rounded-xl p-4 transition-all duration-300 group ${site.config_valid
                  ? 'border-edge-subtle hover:border-edge'
                  : 'border-red-500/30 hover:border-red-500/50'
                  }`}
              >
                {/* Edit Mode */}
                {editingSite === site.domain && editForm ? (
                  <div className="space-y-3">
                    {/* Path */}
                    <div>
                      <label className="block text-xs text-content-muted mb-1">Project Path</label>
                      <input
                        type="text"
                        value={editForm.path}
                        onChange={(e) => setEditForm({ ...editForm, path: e.target.value })}
                        className="w-full px-3 py-2 bg-surface border border-edge rounded-lg text-sm"
                        placeholder="Path"
                      />
                    </div>

                    {/* Template */}
                    <div>
                      <label className="block text-xs text-content-muted mb-1">Template</label>
                      <div className="grid grid-cols-2 gap-1">
                        {(Object.keys(TEMPLATE_INFO) as SiteTemplate[]).map((template) => (
                          <button
                            key={template}
                            onClick={() => setEditForm({ ...editForm, template })}
                            className={`flex items-center gap-1 px-2 py-1 rounded text-xs cursor-pointer transition-colors ${editForm.template === template
                              ? 'bg-emerald-600 text-white'
                              : 'bg-surface-inset text-content-secondary hover:bg-hover'
                              }`}
                          >
                            {TEMPLATE_INFO[template].icon}
                            {TEMPLATE_INFO[template].label}
                          </button>
                        ))}
                      </div>
                    </div>

                    {/* PHP Version & Port */}
                    <div className="grid grid-cols-2 gap-2">
                      {/* PHP Version */}
                      <div>
                        <label className="block text-xs text-content-muted mb-1">PHP Version</label>
                        {!PHP_TEMPLATES.includes(editForm.template as SiteTemplate) ? (
                          <p className="text-xs text-content-muted py-2">Not needed</p>
                        ) : phpVersions.length === 0 ? (
                          <p className="text-xs text-amber-500 py-1">No PHP installed</p>
                        ) : (
                          <select
                            value={editForm.php_version || ''}
                            onChange={(e) => setEditForm({ ...editForm, php_version: e.target.value || undefined })}
                            className="w-full px-2 py-1.5 bg-surface border border-edge rounded-lg text-sm"
                          >
                            <option value="">Select PHP</option>
                            {phpVersions.map((v) => (
                              <option key={v} value={v}>PHP {v}</option>
                            ))}
                          </select>
                        )}
                      </div>

                      {/* Port */}
                      <div>
                        <label className="block text-xs text-content-muted mb-1">Port</label>
                        <input
                          type="number"
                          value={editForm.port}
                          onChange={(e) => setEditForm({ ...editForm, port: parseInt(e.target.value) || 80 })}
                          className="w-full px-2 py-1.5 bg-surface border border-edge rounded-lg text-sm"
                        />
                      </div>
                    </div>

                    {/* Web Server & SSL */}
                    <div className="grid grid-cols-2 gap-2">
                      {/* Web Server */}
                      <div>
                        <label className="block text-xs text-content-muted mb-1">Web Server</label>
                        <div className="flex gap-1">
                          {availableWebServers.map((ws) => (
                            <button
                              key={ws}
                              onClick={() => setEditForm({ ...editForm, web_server: ws })}
                              className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded text-xs cursor-pointer transition-colors ${editForm.web_server === ws
                                ? ws === 'nginx' ? 'bg-green-600 text-white' : 'bg-orange-600 text-white'
                                : 'bg-surface-inset text-content-secondary hover:bg-hover'
                                }`}
                            >
                              {WEB_SERVER_INFO[ws].icon}
                              {WEB_SERVER_INFO[ws].label}
                            </button>
                          ))}
                        </div>
                      </div>

                      {/* SSL Toggle */}
                      <div className="flex flex-col gap-1 pb-1">
                        <label className={`flex items-center gap-2 ${sslReady || editForm.ssl_enabled ? 'cursor-pointer' : 'cursor-not-allowed'}`}>
                          <input
                            type="checkbox"
                            checked={editForm.ssl_enabled}
                            disabled={!sslReady && !editForm.ssl_enabled}
                            onChange={(e) => setEditForm({ ...editForm, ssl_enabled: e.target.checked })}
                            className="w-4 h-4 rounded border-edge bg-surface text-emerald-500 disabled:opacity-50"
                          />
                          <span className={`text-xs flex items-center gap-1 ${sslReady || editForm.ssl_enabled ? 'text-content-secondary' : 'text-content-muted'}`}>
                            <Shield size={12} className={editForm.ssl_enabled ? 'text-emerald-500' : 'text-amber-500'} />
                            SSL (HTTPS)
                          </span>
                        </label>
                        {!sslReady && !editForm.ssl_enabled && (
                          <p className="text-xs text-amber-400 flex items-center gap-1">
                            <AlertTriangle size={10} />
                            Setup SSL in Settings
                          </p>
                        )}
                      </div>
                    </div>

                    {/* Actions */}
                    <div className="flex gap-2 pt-1">
                      <button
                        onClick={() => { setEditingSite(null); setEditForm(null); }}
                        className="flex-1 px-3 py-1.5 bg-surface-inset hover:bg-hover rounded-lg text-sm transition-colors cursor-pointer"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={handleUpdateSite}
                        disabled={processing === site.domain}
                        className="flex-1 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm disabled:opacity-50 transition-colors cursor-pointer"
                      >
                        {processing === site.domain ? <Loader2 size={14} className="animate-spin mx-auto" /> : 'Save'}
                      </button>
                    </div>
                  </div>
                ) : (
                  <>
                    {/* Header */}
                    <div className="flex justify-between items-start mb-3">
                      <div className="flex items-center gap-2">
                        <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${site.config_valid ? 'bg-emerald-500/10' : 'bg-red-500/10'
                          }`}>
                          {site.config_valid
                            ? <Globe size={16} className="text-emerald-500" />
                            : <XCircle size={16} className="text-red-500" />
                          }
                        </div>
                        <div>
                          <h3 className="font-semibold text-sm flex items-center gap-1">
                            {site.ssl_enabled && <Shield size={12} className="text-amber-500" />}
                            {site.domain}
                          </h3>
                          <span className="text-xs text-content-muted font-mono">
                            :{site.port}
                            {site.php_version && ` · PHP ${site.php_version}`}
                          </span>
                        </div>
                      </div>
                      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button
                          onClick={() => startEditing(site)}
                          className="p-1.5 rounded-lg hover:bg-hover text-content-muted transition-colors cursor-pointer"
                          title="Edit"
                        >
                          <Settings2 size={14} />
                        </button>
                        <button
                          onClick={() => handleRegenerateConfig(site.domain, site.web_server)}
                          disabled={processing === site.domain}
                          className="p-1.5 rounded-lg hover:bg-hover text-content-muted transition-colors cursor-pointer disabled:opacity-50"
                          title="Regenerate Config"
                        >
                          <RefreshCw size={14} className={processing === site.domain ? 'animate-spin' : ''} />
                        </button>
                        <button
                          onClick={() => handleDeleteSite(site.domain, site.web_server)}
                          disabled={processing === site.domain}
                          className="p-1.5 rounded-lg hover:bg-red-500/20 hover:text-red-500 text-content-muted transition-colors cursor-pointer disabled:opacity-50"
                          title="Delete"
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </div>

                    {/* Template & Web Server Badges */}
                    <div className="mb-2 flex flex-wrap gap-1">
                      {/* Web Server Badge */}
                      {site.web_server && (
                        <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs ${WEB_SERVER_INFO[site.web_server]?.color || 'bg-surface-inset text-content-secondary'}`}>
                          {WEB_SERVER_INFO[site.web_server]?.icon}
                          {WEB_SERVER_INFO[site.web_server]?.label}
                        </span>
                      )}
                      {/* Template Badge */}
                      {site.template && (
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 bg-surface-inset rounded text-xs text-content-secondary">
                          {TEMPLATE_INFO[site.template]?.icon}
                          {TEMPLATE_INFO[site.template]?.label || site.template}
                        </span>
                      )}
                    </div>

                    {/* Path */}
                    <div className="text-xs text-content-secondary mb-3 truncate" title={site.path}>
                      {site.path}
                    </div>

                    {/* Config Warning */}
                    {!site.config_valid && (
                      <div className="flex items-center gap-1 text-xs text-red-400 mb-3">
                        <AlertTriangle size={12} />
                        Config invalid - click refresh to regenerate
                      </div>
                    )}

                    {/* Open Link */}
                    <div className="flex items-center gap-4 mt-1">
                      <button
                        onClick={() => shellOpen(`${site.ssl_enabled ? 'https' : 'http'}://${site.domain}${site.port !== 80 && site.port !== 443 ? ':' + site.port : ''}`)}
                        className="flex items-center gap-1 text-xs text-blue-500 hover:text-blue-400 transition-colors cursor-pointer"
                      >
                        <ExternalLink size={12} />
                        Local Admin
                      </button>

                      {site.dev_command && (
                        <>
                          <div className="h-3 w-px bg-edge" />
                          {appStatus[site.domain] === 'running' ? (
                            <button
                              onClick={() => handleStopApp(site.domain)}
                              disabled={appProcessing === site.domain}
                              className="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 transition-colors cursor-pointer disabled:opacity-50"
                              title={`Stop: ${site.dev_command}`}
                            >
                              {appProcessing === site.domain ? <Loader2 size={12} className="animate-spin" /> : <Square size={12} />}
                              Stop App
                            </button>
                          ) : (
                            <button
                              onClick={() => handleStartApp(site.domain)}
                              disabled={appProcessing === site.domain}
                              className="flex items-center gap-1 text-xs text-emerald-500 hover:text-emerald-400 transition-colors cursor-pointer disabled:opacity-50"
                              title={`Start: ${site.dev_command}`}
                            >
                              {appProcessing === site.domain ? <Loader2 size={12} className="animate-spin" /> : <Play size={12} />}
                              Start App
                            </button>
                          )}
                        </>
                      )}

                      <div className="h-3 w-px bg-edge" />

                      {activeTunnel?.domain === site.domain ? (
                        <>
                          <button
                            onClick={() => shellOpen(activeTunnel.url)}
                            className="flex items-center gap-1 text-xs text-purple-400 hover:text-purple-300 font-mono transition-colors cursor-pointer"
                            title="Open public URL"
                          >
                            <Globe size={12} />
                            {activeTunnel.url}
                          </button>
                          <button
                            onClick={handleStopTunnel}
                            disabled={tunnelingDomain === 'stopping'}
                            className="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 transition-colors cursor-pointer"
                          >
                            {tunnelingDomain === 'stopping' ? <Loader2 size={12} className="animate-spin" /> : <XCircle size={12} />}
                            Stop
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={() => handleStartTunnel(site)}
                          disabled={tunnelingDomain !== null || activeTunnel !== null}
                          className="flex items-center gap-1 text-xs text-emerald-500 hover:text-emerald-400 transition-colors cursor-pointer disabled:opacity-50"
                        >
                          {tunnelingDomain === site.domain ? <Loader2 size={12} className="animate-spin" /> : <Globe size={12} />}
                          Share URL
                        </button>
                      )}
                    </div>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
