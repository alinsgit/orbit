import { useState, useEffect } from 'react';
import {
  FolderOpen, Plus, Trash2, Globe, ExternalLink, Loader2, Shield,
  AlertTriangle, RefreshCw, CheckCircle2, XCircle, Settings2,
  FileCode, Layers, Database, ShoppingCart
} from 'lucide-react';
import {
  getSites, createSite, deleteSite, updateSite, regenerateSiteConfig,
  generateSslCert, nginxTestConfig, nginxReload, nginxStatus,
  addHostElevated, SiteWithStatus, Site, WebServer, reloadService
} from '../lib/api';
import { useApp } from '../lib/AppContext';
import { open } from '@tauri-apps/plugin-dialog';
import { open as shellOpen } from '@tauri-apps/plugin-shell';

type SiteTemplate = 'http' | 'laravel' | 'wordpress' | 'litecart' | 'static';

const WEB_SERVER_INFO: Record<WebServer, { label: string; icon: string; color: string }> = {
  nginx: { label: 'Nginx', icon: '🌐', color: 'bg-green-500/10 text-green-400' },
  apache: { label: 'Apache', icon: '🪶', color: 'bg-orange-500/10 text-orange-400' }
};

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
  }
};

export function SitesManager() {
  const { getInstalledPhpVersions, services } = useApp();

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

  useEffect(() => {
    refreshSites();
  }, []);

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

  // Handle add site
  const handleAddSite = async () => {
    if (!newSite.domain || !newSite.path) {
      alert('Domain and path are required');
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
    } catch (e: any) {
      console.error(e);
      alert('Failed to create site: ' + e);
    } finally {
      setLoading(false);
    }
  };

  // Handle delete site
  const handleDeleteSite = async (domain: string, webServer?: WebServer) => {
    if (!confirm(`Are you sure you want to delete ${domain}?`)) {
      return;
    }

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
      alert('Failed to delete site: ' + e);
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
    } catch (e: any) {
      console.error(e);
      alert('Failed to update site: ' + e);
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
      await refreshSites();
    } catch (e: any) {
      console.error(e);
      alert('Failed to regenerate config: ' + e);
    } finally {
      setProcessing(null);
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
          {/* Nginx Status & Reload */}
          <div className="flex items-center gap-2 px-3 py-2 bg-surface-raised rounded-lg border border-edge">
            <div className={`w-2 h-2 rounded-full ${isNginxRunning ? 'bg-emerald-500' : 'bg-red-500'}`} />
            <span className="text-sm text-content-secondary">Nginx</span>
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
            className="p-2 bg-surface-raised hover:bg-hover rounded-lg transition-colors cursor-pointer disabled:opacity-50"
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
        <div className={`mb-4 p-3 rounded-lg flex items-center justify-between ${
          nginxMessage.type === 'success'
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
              >
                <FolderOpen size={16} />
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
                    onClick={() => setNewSite({ ...newSite, template })}
                    className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors cursor-pointer ${
                      newSite.template === template
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
              {newSite.template === 'static' ? (
                <p className="text-sm text-content-muted py-2">Not needed for static sites</p>
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
                      className={`flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors cursor-pointer ${
                        newSite.web_server === ws
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
            <div className="flex items-end pb-1">
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="ssl_enabled"
                  checked={newSite.ssl_enabled}
                  onChange={(e) => setNewSite({ ...newSite, ssl_enabled: e.target.checked })}
                  className="w-4 h-4 rounded border-edge bg-surface text-emerald-500 focus:ring-emerald-500 focus:ring-offset-0"
                />
                <label htmlFor="ssl_enabled" className="text-sm text-content-secondary flex items-center gap-2">
                  <Shield size={14} className="text-emerald-500" />
                  Enable SSL (HTTPS)
                </label>
              </div>
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
                className={`bg-surface-raised border rounded-xl p-4 transition-all duration-300 group ${
                  site.config_valid
                    ? 'border-edge-subtle hover:border-edge'
                    : 'border-red-500/30 hover:border-red-500/50'
                }`}
              >
                {/* Edit Mode */}
                {editingSite === site.domain && editForm ? (
                  <div className="space-y-3">
                    <input
                      type="text"
                      value={editForm.path}
                      onChange={(e) => setEditForm({ ...editForm, path: e.target.value })}
                      className="w-full px-3 py-2 bg-surface border border-edge rounded-lg text-sm"
                      placeholder="Path"
                    />
                    <div className="grid grid-cols-2 gap-2">
                      {(Object.keys(TEMPLATE_INFO) as SiteTemplate[]).map((template) => (
                        <button
                          key={template}
                          onClick={() => setEditForm({ ...editForm, template })}
                          className={`flex items-center gap-1 px-2 py-1 rounded text-xs ${
                            editForm.template === template
                              ? 'bg-emerald-600 text-white'
                              : 'bg-surface-inset text-content-secondary'
                          }`}
                        >
                          {TEMPLATE_INFO[template].icon}
                          {TEMPLATE_INFO[template].label}
                        </button>
                      ))}
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={() => { setEditingSite(null); setEditForm(null); }}
                        className="flex-1 px-3 py-1.5 bg-surface-inset rounded-lg text-sm"
                      >
                        Cancel
                      </button>
                      <button
                        onClick={handleUpdateSite}
                        disabled={processing === site.domain}
                        className="flex-1 px-3 py-1.5 bg-emerald-600 rounded-lg text-sm disabled:opacity-50"
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
                        <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${
                          site.config_valid ? 'bg-emerald-500/10' : 'bg-red-500/10'
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
                    <button
                      onClick={() => shellOpen(`${site.ssl_enabled ? 'https' : 'http'}://${site.domain}${site.port !== 80 && site.port !== 443 ? ':' + site.port : ''}`)}
                      className="flex items-center gap-1 text-xs text-emerald-500 hover:text-emerald-400 transition-colors cursor-pointer"
                    >
                      <ExternalLink size={12} />
                      Open in browser
                    </button>
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
