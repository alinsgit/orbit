import { useState, useEffect } from 'react';
import { X, Loader2, ToggleLeft, ToggleRight, Save, Cpu, Server, Gauge, RefreshCw, ChevronDown, ChevronRight, FileCode, RotateCcw, Trash2, Plus } from 'lucide-react';
import {
  getPhpConfig, setPhpExtension, setPhpSetting, PhpConfig,
  getOpcacheConfig, setOpcacheConfig as updateOpcacheConfig, OpcacheConfig,
  getNginxGzipConfig, setNginxGzipConfig as updateNginxGzipConfig, NginxGzipConfig,
  listTemplates, getTemplate, saveTemplate, resetTemplate, deleteTemplate, TemplateInfo
} from '../lib/api';
import { useApp } from '../lib/AppContext';

interface ServiceConfigDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  serviceName: string;
  serviceType: string;
  serviceVersion: string;
}

export function ServiceConfigDrawer({ isOpen, onClose, serviceName, serviceType, serviceVersion }: ServiceConfigDrawerProps) {
  const { addToast } = useApp();

  // PHP State
  const [phpConfig, setPhpConfig] = useState<PhpConfig | null>(null);
  const [opcacheConfig, setOpcacheConfig] = useState<OpcacheConfig | null>(null);
  const [phpLoading, setPhpLoading] = useState(false);
  const [saving, setSaving] = useState<string | null>(null);

  // Nginx State
  const [nginxGzipConfig, setNginxGzipConfigState] = useState<NginxGzipConfig | null>(null);
  const [nginxLoading, setNginxLoading] = useState(false);

  // Template State
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [templateContent, setTemplateContent] = useState<string>('');
  const [templateLoading, setTemplateLoading] = useState(false);
  const [templateSaving, setTemplateSaving] = useState(false);
  const [newTemplateName, setNewTemplateName] = useState('');
  const [showNewTemplateInput, setShowNewTemplateInput] = useState(false);

  // UI State
  const [expandedSections, setExpandedSections] = useState<Record<string, boolean>>({
    extensions: true,
    settings: false,
    opcache: true,
    gzip: true,
    templates: true,
  });

  useEffect(() => {
    if (isOpen) {
      loadConfig();
    }
  }, [isOpen, serviceName, serviceType]);

  const loadConfig = async () => {
    if (serviceType === 'php') {
      await loadPhpConfig();
    } else if (serviceType === 'nginx') {
      await loadNginxConfig();
    }
  };

  const loadPhpConfig = async () => {
    setPhpLoading(true);
    try {
      const [config, opcache] = await Promise.all([
        getPhpConfig(serviceVersion),
        getOpcacheConfig(serviceVersion)
      ]);
      setPhpConfig(config);
      setOpcacheConfig(opcache);
    } catch (e) {
      console.error('Failed to load PHP config:', e);
      addToast({ type: 'error', message: 'Failed to load PHP configuration' });
    } finally {
      setPhpLoading(false);
    }
  };

  const loadNginxConfig = async () => {
    setNginxLoading(true);
    try {
      const [config, templateList] = await Promise.all([
        getNginxGzipConfig(),
        listTemplates()
      ]);
      setNginxGzipConfigState(config);
      setTemplates(templateList);
    } catch (e) {
      console.error('Failed to load Nginx config:', e);
    } finally {
      setNginxLoading(false);
    }
  };

  const loadTemplateContent = async (name: string) => {
    setTemplateLoading(true);
    try {
      const content = await getTemplate(name);
      setTemplateContent(content);
      setSelectedTemplate(name);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to load template: ${e}` });
    } finally {
      setTemplateLoading(false);
    }
  };

  const handleSaveTemplate = async () => {
    if (!selectedTemplate) return;
    setTemplateSaving(true);
    try {
      await saveTemplate(selectedTemplate, templateContent);
      addToast({ type: 'success', message: `Template ${selectedTemplate} saved successfully` });
      // Refresh templates list
      const templateList = await listTemplates();
      setTemplates(templateList);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to save template: ${e}` });
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleResetTemplate = async () => {
    if (!selectedTemplate) return;
    setTemplateSaving(true);
    try {
      await resetTemplate(selectedTemplate);
      // Reload the template content
      const content = await getTemplate(selectedTemplate);
      setTemplateContent(content);
      addToast({ type: 'success', message: `Template ${selectedTemplate} reset to default` });
      // Refresh templates list
      const templateList = await listTemplates();
      setTemplates(templateList);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to reset template: ${e}` });
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleDeleteTemplate = async () => {
    if (!selectedTemplate) return;
    const templateInfo = templates.find(t => t.name === selectedTemplate);
    if (!templateInfo?.is_custom) {
      addToast({ type: 'error', message: 'Cannot delete built-in templates' });
      return;
    }
    setTemplateSaving(true);
    try {
      await deleteTemplate(selectedTemplate);
      setSelectedTemplate(null);
      setTemplateContent('');
      addToast({ type: 'success', message: `Template ${selectedTemplate} deleted` });
      // Refresh templates list
      const templateList = await listTemplates();
      setTemplates(templateList);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to delete template: ${e}` });
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleCreateTemplate = async () => {
    if (!newTemplateName.trim()) return;
    const name = newTemplateName.trim().toLowerCase().replace(/\s+/g, '-');
    setTemplateSaving(true);
    try {
      // Create with a basic template
      const defaultContent = `# Custom Nginx Template: ${name}
# Variables: {{DOMAIN}}, {{ROOT_PATH}}, {{PORT}}, {{PHP_PORT}}, {{SSL_CERT}}, {{SSL_KEY}}

server {
    listen {{PORT}};
    server_name {{DOMAIN}};
    root {{ROOT_PATH}};
    index index.php index.html;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    location ~ \\.php$ {
        fastcgi_pass 127.0.0.1:{{PHP_PORT}};
        fastcgi_index index.php;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        include fastcgi_params;
    }
}
`;
      await saveTemplate(name, defaultContent);
      setNewTemplateName('');
      setShowNewTemplateInput(false);
      addToast({ type: 'success', message: `Template ${name} created` });
      // Refresh and select the new template
      const templateList = await listTemplates();
      setTemplates(templateList);
      await loadTemplateContent(name);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to create template: ${e}` });
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleToggleExtension = async (extName: string, currentEnabled: boolean) => {
    setSaving(extName);
    try {
      await setPhpExtension(serviceVersion, extName, !currentEnabled);
      await loadPhpConfig();
      addToast({
        type: 'success',
        message: `Extension ${extName} ${!currentEnabled ? 'enabled' : 'disabled'}. Restart PHP to apply.`
      });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to toggle extension: ${e}` });
    } finally {
      setSaving(null);
    }
  };

  const handleUpdateSetting = async (key: string, value: string) => {
    setSaving(key);
    try {
      await setPhpSetting(serviceVersion, key, value);
      await loadPhpConfig();
      addToast({ type: 'success', message: `Setting ${key} updated. Restart PHP to apply.` });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to update setting: ${e}` });
    } finally {
      setSaving(null);
    }
  };

  const handleToggleOpcache = async () => {
    if (!opcacheConfig) return;
    setSaving('opcache');
    try {
      const newConfig = { ...opcacheConfig, enabled: !opcacheConfig.enabled };
      await updateOpcacheConfig(serviceVersion, newConfig);
      setOpcacheConfig(newConfig);
      addToast({ type: 'success', message: 'OPcache configuration updated. Restart PHP to apply.' });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to update OPcache: ${e}` });
    } finally {
      setSaving(null);
    }
  };

  const handleUpdateOpcacheMemory = async (memory: string) => {
    if (!opcacheConfig) return;
    setSaving('opcache-memory');
    try {
      const newConfig = { ...opcacheConfig, memory };
      await updateOpcacheConfig(serviceVersion, newConfig);
      setOpcacheConfig(newConfig);
      addToast({ type: 'success', message: 'OPcache memory updated. Restart PHP to apply.' });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to update OPcache: ${e}` });
    } finally {
      setSaving(null);
    }
  };

  const handleToggleNginxGzip = async () => {
    if (!nginxGzipConfig) return;
    setSaving('gzip');
    try {
      const newConfig = { ...nginxGzipConfig, enabled: !nginxGzipConfig.enabled };
      await updateNginxGzipConfig(newConfig);
      setNginxGzipConfigState(newConfig);
      addToast({ type: 'success', message: 'Nginx gzip configuration updated.' });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to update gzip: ${e}` });
    } finally {
      setSaving(null);
    }
  };

  const handleUpdateGzipLevel = async (level: number) => {
    if (!nginxGzipConfig) return;
    setSaving('gzip-level');
    try {
      const newConfig = { ...nginxGzipConfig, level };
      await updateNginxGzipConfig(newConfig);
      setNginxGzipConfigState(newConfig);
      addToast({ type: 'success', message: 'Gzip level updated.' });
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to update gzip level: ${e}` });
    } finally {
      setSaving(null);
    }
  };

  const toggleSection = (section: string) => {
    setExpandedSections(prev => ({ ...prev, [section]: !prev[section] }));
  };

  // Group extensions by category
  const groupExtensions = (extensions: PhpConfig['extensions']) => {
    const groups: Record<string, typeof extensions> = {
      'Database': [],
      'Web': [],
      'Compression': [],
      'Security': [],
      'Other': [],
    };

    const categoryMap: Record<string, string> = {
      mysqli: 'Database', pdo_mysql: 'Database', pdo_pgsql: 'Database',
      pdo_sqlite: 'Database', sqlite3: 'Database',
      curl: 'Web', soap: 'Web', sockets: 'Web', gd: 'Web',
      mbstring: 'Web', intl: 'Web',
      openssl: 'Security', sodium: 'Security',
      zip: 'Compression', zlib: 'Compression', bz2: 'Compression',
      fileinfo: 'Other', exif: 'Other',
    };

    for (const ext of extensions) {
      const category = categoryMap[ext.name] || 'Other';
      groups[category].push(ext);
    }

    return Object.fromEntries(
      Object.entries(groups).filter(([, exts]) => exts.length > 0)
    );
  };

  if (!isOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/50 z-40 transition-opacity"
        onClick={onClose}
      />

      {/* Drawer */}
      <div className="fixed right-0 top-0 h-full w-[480px] bg-surface border-l border-edge z-50 shadow-2xl overflow-hidden flex flex-col">
        {/* Header */}
        <div className="p-4 border-b border-edge flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-surface-raised flex items-center justify-center text-xl">
              {serviceType === 'php' ? '🐘' : serviceType === 'nginx' ? '🌐' : '⚙️'}
            </div>
            <div>
              <h2 className="font-semibold">{serviceName}</h2>
              <p className="text-xs text-content-muted">Configuration</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={loadConfig}
              className="p-2 hover:bg-hover rounded-lg transition-colors"
              title="Refresh"
            >
              <RefreshCw size={16} />
            </button>
            <button
              onClick={onClose}
              className="p-2 hover:bg-hover rounded-lg transition-colors"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* PHP Configuration */}
          {serviceType === 'php' && (
            <>
              {phpLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="animate-spin text-emerald-500" size={32} />
                </div>
              ) : phpConfig ? (
                <>
                  {/* OPcache Section */}
                  {opcacheConfig && (
                    <div className="border border-edge-subtle rounded-lg overflow-hidden">
                      <button
                        onClick={() => toggleSection('opcache')}
                        className="w-full px-4 py-3 flex items-center justify-between bg-surface-raised hover:bg-hover transition-colors"
                      >
                        <div className="flex items-center gap-2">
                          <Gauge size={16} className="text-orange-500" />
                          <span className="font-medium">OPcache</span>
                          <span className={`text-xs px-2 py-0.5 rounded ${opcacheConfig.enabled ? 'bg-emerald-500/20 text-emerald-400' : 'bg-surface-inset text-content-secondary'}`}>
                            {opcacheConfig.enabled ? 'ON' : 'OFF'}
                          </span>
                        </div>
                        {expandedSections.opcache ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                      </button>

                      {expandedSections.opcache && (
                        <div className="p-4 space-y-4">
                          <div className="flex items-center justify-between">
                            <div>
                              <div className="font-medium text-sm">Enable OPcache</div>
                              <div className="text-xs text-content-muted">Bytecode caching for faster execution</div>
                            </div>
                            <button
                              onClick={handleToggleOpcache}
                              disabled={saving === 'opcache'}
                              className={`p-2 rounded-lg transition-colors ${opcacheConfig.enabled ? 'bg-emerald-500/20 text-emerald-400' : 'bg-surface-inset text-content-secondary'}`}
                            >
                              {saving === 'opcache' ? (
                                <Loader2 size={18} className="animate-spin" />
                              ) : opcacheConfig.enabled ? (
                                <ToggleRight size={18} />
                              ) : (
                                <ToggleLeft size={18} />
                              )}
                            </button>
                          </div>

                          {opcacheConfig.enabled && (
                            <div>
                              <label className="block text-xs text-content-muted mb-1">Memory (MB)</label>
                              <select
                                value={opcacheConfig.memory}
                                onChange={(e) => handleUpdateOpcacheMemory(e.target.value)}
                                disabled={saving === 'opcache-memory'}
                                className="w-full px-3 py-2 bg-surface-raised border border-edge rounded-lg text-sm"
                              >
                                <option value="64">64 MB</option>
                                <option value="128">128 MB (Recommended)</option>
                                <option value="256">256 MB</option>
                                <option value="512">512 MB</option>
                              </select>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  )}

                  {/* Extensions Section */}
                  <div className="border border-edge-subtle rounded-lg overflow-hidden">
                    <button
                      onClick={() => toggleSection('extensions')}
                      className="w-full px-4 py-3 flex items-center justify-between bg-surface-raised hover:bg-hover transition-colors"
                    >
                      <div className="flex items-center gap-2">
                        <Cpu size={16} className="text-emerald-500" />
                        <span className="font-medium">Extensions</span>
                        <span className="text-xs text-content-muted bg-surface-inset px-2 py-0.5 rounded">
                          {phpConfig.extensions.filter(e => e.enabled).length}/{phpConfig.extensions.length}
                        </span>
                      </div>
                      {expandedSections.extensions ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    </button>

                    {expandedSections.extensions && (
                      <div className="p-4 space-y-4">
                        {Object.entries(groupExtensions(phpConfig.extensions)).map(([category, exts]) => (
                          <div key={category}>
                            <h4 className="text-xs font-medium text-content-muted uppercase mb-2">{category}</h4>
                            <div className="grid grid-cols-2 gap-2">
                              {exts.map(ext => (
                                <button
                                  key={ext.name}
                                  onClick={() => handleToggleExtension(ext.name, ext.enabled)}
                                  disabled={saving === ext.name}
                                  className={`flex items-center justify-between px-3 py-2 rounded-lg border text-sm transition-colors ${
                                    ext.enabled
                                      ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
                                      : 'bg-surface-raised border-edge text-content-secondary hover:border-edge-subtle'
                                  }`}
                                >
                                  <span className="truncate">{ext.name}</span>
                                  {saving === ext.name ? (
                                    <Loader2 size={14} className="animate-spin" />
                                  ) : ext.enabled ? (
                                    <ToggleRight size={16} className="text-emerald-500" />
                                  ) : (
                                    <ToggleLeft size={16} className="text-content-muted" />
                                  )}
                                </button>
                              ))}
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Settings Section */}
                  <div className="border border-edge-subtle rounded-lg overflow-hidden">
                    <button
                      onClick={() => toggleSection('settings')}
                      className="w-full px-4 py-3 flex items-center justify-between bg-surface-raised hover:bg-hover transition-colors"
                    >
                      <div className="flex items-center gap-2">
                        <Server size={16} className="text-blue-500" />
                        <span className="font-medium">PHP Settings</span>
                      </div>
                      {expandedSections.settings ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    </button>

                    {expandedSections.settings && (
                      <div className="p-4 space-y-3">
                        {[
                          { key: 'memory_limit', label: 'Memory Limit', placeholder: '256M' },
                          { key: 'upload_max_filesize', label: 'Max Upload Size', placeholder: '64M' },
                          { key: 'post_max_size', label: 'Max POST Size', placeholder: '64M' },
                          { key: 'max_execution_time', label: 'Max Execution Time', placeholder: '30' },
                          { key: 'display_errors', label: 'Display Errors', placeholder: 'On' },
                          { key: 'date.timezone', label: 'Timezone', placeholder: 'Europe/Istanbul' },
                        ].map(({ key, label, placeholder }) => (
                          <div key={key}>
                            <label className="block text-xs text-content-muted mb-1">{label}</label>
                            <div className="flex gap-2">
                              <input
                                type="text"
                                value={phpConfig.settings[key] || ''}
                                placeholder={placeholder}
                                onChange={(e) => {
                                  setPhpConfig({
                                    ...phpConfig,
                                    settings: { ...phpConfig.settings, [key]: e.target.value }
                                  });
                                }}
                                className="flex-1 px-3 py-2 bg-surface-raised border border-edge rounded-lg text-sm"
                              />
                              <button
                                onClick={() => handleUpdateSetting(key, phpConfig.settings[key] || '')}
                                disabled={saving === key}
                                className="px-3 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-neutral-700 rounded-lg transition-colors"
                              >
                                {saving === key ? (
                                  <Loader2 size={14} className="animate-spin" />
                                ) : (
                                  <Save size={14} />
                                )}
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </>
              ) : (
                <div className="text-center py-8 text-content-muted">
                  Failed to load PHP configuration
                </div>
              )}
            </>
          )}

          {/* Nginx Configuration */}
          {serviceType === 'nginx' && (
            <>
              {nginxLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="animate-spin text-emerald-500" size={32} />
                </div>
              ) : (
                <>
                  {/* Gzip Section */}
                  {nginxGzipConfig && (
                    <div className="border border-edge-subtle rounded-lg overflow-hidden">
                      <button
                        onClick={() => toggleSection('gzip')}
                        className="w-full px-4 py-3 flex items-center justify-between bg-surface-raised hover:bg-hover transition-colors"
                      >
                        <div className="flex items-center gap-2">
                          <svg className="w-4 h-4 text-green-500" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
                          </svg>
                          <span className="font-medium">Gzip Compression</span>
                          <span className={`text-xs px-2 py-0.5 rounded ${nginxGzipConfig.enabled ? 'bg-emerald-500/20 text-emerald-400' : 'bg-surface-inset text-content-secondary'}`}>
                            {nginxGzipConfig.enabled ? 'ON' : 'OFF'}
                          </span>
                        </div>
                        {expandedSections.gzip ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                      </button>

                      {expandedSections.gzip && (
                        <div className="p-4 space-y-4">
                          <div className="flex items-center justify-between">
                            <div>
                              <div className="font-medium text-sm">Enable Gzip</div>
                              <div className="text-xs text-content-muted">Compress responses for faster transfer</div>
                            </div>
                            <button
                              onClick={handleToggleNginxGzip}
                              disabled={saving === 'gzip'}
                              className={`p-2 rounded-lg transition-colors ${nginxGzipConfig.enabled ? 'bg-emerald-500/20 text-emerald-400' : 'bg-surface-inset text-content-secondary'}`}
                            >
                              {saving === 'gzip' ? (
                                <Loader2 size={18} className="animate-spin" />
                              ) : nginxGzipConfig.enabled ? (
                                <ToggleRight size={18} />
                              ) : (
                                <ToggleLeft size={18} />
                              )}
                            </button>
                          </div>

                          {nginxGzipConfig.enabled && (
                            <div>
                              <label className="block text-xs text-content-muted mb-1">Compression Level</label>
                              <select
                                value={nginxGzipConfig.level}
                                onChange={(e) => handleUpdateGzipLevel(parseInt(e.target.value))}
                                disabled={saving === 'gzip-level'}
                                className="w-full px-3 py-2 bg-surface-raised border border-edge rounded-lg text-sm"
                              >
                                <option value="1">1 (Fastest)</option>
                                <option value="3">3</option>
                                <option value="6">6 (Balanced)</option>
                                <option value="9">9 (Best Compression)</option>
                              </select>
                            </div>
                          )}

                          <p className="text-xs text-content-muted">
                            Nginx will be reloaded automatically when changes are saved.
                          </p>
                        </div>
                      )}
                    </div>
                  )}

                  {/* Templates Section */}
                  <div className="border border-edge-subtle rounded-lg overflow-hidden">
                    <button
                      onClick={() => toggleSection('templates')}
                      className="w-full px-4 py-3 flex items-center justify-between bg-surface-raised hover:bg-hover transition-colors"
                    >
                      <div className="flex items-center gap-2">
                        <FileCode size={16} className="text-blue-500" />
                        <span className="font-medium">Site Templates</span>
                        <span className="text-xs text-content-muted bg-surface-inset px-2 py-0.5 rounded">
                          {templates.length}
                        </span>
                      </div>
                      {expandedSections.templates ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    </button>

                    {expandedSections.templates && (
                      <div className="p-4 space-y-4">
                        {/* Template List */}
                        <div>
                          <div className="flex items-center justify-between mb-2">
                            <label className="block text-xs text-content-muted">Select Template</label>
                            <button
                              onClick={() => setShowNewTemplateInput(true)}
                              className="text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1"
                            >
                              <Plus size={12} />
                              New Template
                            </button>
                          </div>

                          {/* New Template Input */}
                          {showNewTemplateInput && (
                            <div className="flex gap-2 mb-2">
                              <input
                                type="text"
                                value={newTemplateName}
                                onChange={(e) => setNewTemplateName(e.target.value)}
                                placeholder="template-name"
                                className="flex-1 px-3 py-2 bg-surface-raised border border-edge rounded-lg text-sm"
                                onKeyDown={(e) => e.key === 'Enter' && handleCreateTemplate()}
                              />
                              <button
                                onClick={handleCreateTemplate}
                                disabled={templateSaving || !newTemplateName.trim()}
                                className="px-3 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 rounded-lg transition-colors"
                              >
                                {templateSaving ? <Loader2 size={14} className="animate-spin" /> : <Plus size={14} />}
                              </button>
                              <button
                                onClick={() => { setShowNewTemplateInput(false); setNewTemplateName(''); }}
                                className="px-3 py-2 bg-surface-inset hover:bg-hover rounded-lg transition-colors"
                              >
                                <X size={14} />
                              </button>
                            </div>
                          )}

                          {/* Template Buttons */}
                          <div className="grid grid-cols-2 gap-2">
                            {templates.map(template => (
                              <button
                                key={template.name}
                                onClick={() => loadTemplateContent(template.name)}
                                className={`flex items-center justify-between px-3 py-2 rounded-lg border text-sm transition-colors ${
                                  selectedTemplate === template.name
                                    ? 'bg-blue-500/10 border-blue-500/30 text-blue-400'
                                    : 'bg-surface-raised border-edge text-content-secondary hover:border-edge-subtle'
                                }`}
                              >
                                <span className="truncate">{template.name}</span>
                                {template.is_custom && (
                                  <span className="text-[10px] text-orange-400 ml-1">custom</span>
                                )}
                              </button>
                            ))}
                          </div>
                        </div>

                        {/* Template Editor */}
                        {selectedTemplate && (
                          <div className="space-y-3">
                            <div className="flex items-center justify-between">
                              <div className="flex items-center gap-2">
                                <span className="font-medium text-sm">{selectedTemplate}</span>
                                {templates.find(t => t.name === selectedTemplate)?.is_custom && (
                                  <span className="text-xs bg-orange-500/20 text-orange-400 px-2 py-0.5 rounded">Custom</span>
                                )}
                              </div>
                              <div className="flex gap-1">
                                <button
                                  onClick={handleSaveTemplate}
                                  disabled={templateSaving || templateLoading}
                                  className="p-2 bg-emerald-600 hover:bg-emerald-500 disabled:bg-neutral-700 rounded-lg transition-colors"
                                  title="Save Template"
                                >
                                  {templateSaving ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} />}
                                </button>
                                <button
                                  onClick={handleResetTemplate}
                                  disabled={templateSaving || templateLoading}
                                  className="p-2 bg-surface-inset hover:bg-hover disabled:bg-neutral-800 rounded-lg transition-colors"
                                  title="Reset to Default"
                                >
                                  <RotateCcw size={14} />
                                </button>
                                {templates.find(t => t.name === selectedTemplate)?.is_custom && (
                                  <button
                                    onClick={handleDeleteTemplate}
                                    disabled={templateSaving || templateLoading}
                                    className="p-2 bg-red-600/80 hover:bg-red-500 disabled:bg-neutral-700 rounded-lg transition-colors"
                                    title="Delete Template"
                                  >
                                    <Trash2 size={14} />
                                  </button>
                                )}
                              </div>
                            </div>

                            {templateLoading ? (
                              <div className="flex items-center justify-center py-8">
                                <Loader2 className="animate-spin text-blue-500" size={24} />
                              </div>
                            ) : (
                              <textarea
                                value={templateContent}
                                onChange={(e) => setTemplateContent(e.target.value)}
                                className="w-full h-64 px-3 py-2 bg-surface border border-edge rounded-lg text-sm font-mono resize-y"
                                spellCheck={false}
                              />
                            )}

                            <p className="text-xs text-content-muted">
                              Variables: {'{{DOMAIN}}'}, {'{{ROOT_PATH}}'}, {'{{PORT}}'}, {'{{PHP_PORT}}'}, {'{{SSL_CERT}}'}, {'{{SSL_KEY}}'}
                            </p>
                          </div>
                        )}

                        {!selectedTemplate && templates.length > 0 && (
                          <p className="text-xs text-content-muted text-center py-4">
                            Select a template to edit
                          </p>
                        )}
                      </div>
                    )}
                  </div>

                  {!nginxGzipConfig && templates.length === 0 && (
                    <div className="text-center py-8 text-content-muted">
                      Failed to load Nginx configuration
                    </div>
                  )}
                </>
              )}
            </>
          )}

          {/* Unsupported service type */}
          {serviceType !== 'php' && serviceType !== 'nginx' && (
            <div className="text-center py-8 text-content-muted">
              <p>No configuration available for this service type.</p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-edge text-xs text-content-muted text-center">
          Changes may require service restart to take effect
        </div>
      </div>
    </>
  );
}
