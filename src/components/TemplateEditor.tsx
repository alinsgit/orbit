import { useState, useEffect } from 'react';
import { FileCode, RefreshCw, Save, RotateCcw, Plus, Trash2, AlertCircle, CheckCircle, X } from 'lucide-react';
import {
  listTemplates,
  getTemplate,
  saveTemplate,
  resetTemplate,
  deleteTemplate,
  type TemplateInfo,
} from '../lib/api';

interface TemplateEditorProps {
  onClose?: () => void;
}

export default function TemplateEditor({ onClose }: TemplateEditorProps) {
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [content, setContent] = useState<string>('');
  const [originalContent, setOriginalContent] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [showNewDialog, setShowNewDialog] = useState(false);
  const [newTemplateName, setNewTemplateName] = useState('');

  useEffect(() => {
    loadTemplates();
  }, []);

  const loadTemplates = async () => {
    try {
      setLoading(true);
      setError(null);
      const list = await listTemplates();
      setTemplates(list);

      // Select first template if none selected
      if (!selectedTemplate && list.length > 0) {
        await selectTemplate(list[0].name);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load templates');
    } finally {
      setLoading(false);
    }
  };

  const selectTemplate = async (name: string) => {
    try {
      setError(null);
      const templateContent = await getTemplate(name);
      setSelectedTemplate(name);
      setContent(templateContent);
      setOriginalContent(templateContent);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load template');
    }
  };

  const handleSave = async () => {
    if (!selectedTemplate) return;

    try {
      setSaving(true);
      setError(null);
      await saveTemplate(selectedTemplate, content);
      setOriginalContent(content);
      setSuccess('Template saved successfully');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save template');
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    if (!selectedTemplate) return;

    try {
      setError(null);
      await resetTemplate(selectedTemplate);
      await selectTemplate(selectedTemplate);
      setSuccess('Template reset to default');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reset template');
    }
  };

  const handleDelete = async () => {
    if (!selectedTemplate) return;

    const template = templates.find(t => t.name === selectedTemplate);
    if (!template?.is_custom) {
      setError('Cannot delete default templates');
      return;
    }

    try {
      setError(null);
      await deleteTemplate(selectedTemplate);
      setSelectedTemplate(null);
      setContent('');
      setOriginalContent('');
      await loadTemplates();
      setSuccess('Template deleted');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete template');
    }
  };

  const handleCreateNew = async () => {
    if (!newTemplateName.trim()) return;

    const name = newTemplateName.trim().toLowerCase().replace(/[^a-z0-9-]/g, '-');

    try {
      setError(null);
      // Create with default content
      const defaultContent = `# Custom template: ${name}
server {
    listen       {{port}};
    server_name  {{domain}};
    root         "{{path}}";

    index  index.php index.html index.htm;

    # Logs
    access_log  logs/{{domain}}.access.log;
    error_log   logs/{{domain}}.error.log;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    # PHP-FPM Configuration
    location ~ \\.php$ {
        fastcgi_pass   127.0.0.1:{{php_port}};
        fastcgi_index  index.php;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        include        fastcgi_params;
    }

    # Deny access to hidden files
    location ~ /\\. {
        deny all;
    }
}
`;
      await saveTemplate(name, defaultContent);
      setShowNewDialog(false);
      setNewTemplateName('');
      await loadTemplates();
      await selectTemplate(name);
      setSuccess('Template created');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create template');
    }
  };

  const hasChanges = content !== originalContent;
  const isCustom = templates.find(t => t.name === selectedTemplate)?.is_custom ?? false;

  return (
    <div className="flex flex-col h-full bg-surface">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-edge">
        <div className="flex items-center gap-3">
          <FileCode className="w-6 h-6 text-purple-500" />
          <div>
            <h2 className="text-lg font-semibold text-content">Template Editor</h2>
            <p className="text-xs text-content-muted">Customize Nginx site configurations</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={loadTemplates}
            disabled={loading}
            className="p-2 hover:bg-hover rounded-lg transition-colors disabled:opacity-50"
            title="Refresh"
          >
            <RefreshCw className={`w-5 h-5 ${loading ? 'animate-spin' : ''}`} />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-2 hover:bg-hover rounded-lg transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          )}
        </div>
      </div>

      {/* Messages */}
      {error && (
        <div className="mx-4 mt-4 flex items-center gap-2 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400">
          <AlertCircle className="w-5 h-5 flex-shrink-0" />
          <span className="text-sm">{error}</span>
        </div>
      )}

      {success && (
        <div className="mx-4 mt-4 flex items-center gap-2 p-3 bg-green-500/10 border border-green-500/20 rounded-lg text-green-400">
          <CheckCircle className="w-5 h-5 flex-shrink-0" />
          <span className="text-sm">{success}</span>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        {/* Template List */}
        <div className="w-64 border-r border-edge flex flex-col">
          <div className="p-3 border-b border-edge">
            <button
              onClick={() => setShowNewDialog(true)}
              className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-purple-600 hover:bg-purple-700 rounded-lg transition-colors text-sm font-medium"
            >
              <Plus className="w-4 h-4" />
              New Template
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {templates.map((template) => (
              <button
                key={template.name}
                onClick={() => selectTemplate(template.name)}
                className={`w-full text-left px-3 py-2 rounded-lg transition-colors ${
                  selectedTemplate === template.name
                    ? 'bg-purple-600/20 border border-purple-500/30'
                    : 'hover:bg-hover'
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">{template.name}</span>
                  {template.is_custom && (
                    <span className="text-[10px] px-1.5 py-0.5 bg-blue-500/20 text-blue-400 rounded">
                      Custom
                    </span>
                  )}
                </div>
                <p className="text-xs text-content-muted mt-0.5">{template.description}</p>
              </button>
            ))}
          </div>
        </div>

        {/* Editor */}
        <div className="flex-1 flex flex-col">
          {selectedTemplate ? (
            <>
              {/* Editor Toolbar */}
              <div className="flex items-center justify-between p-3 border-b border-edge bg-surface">
                <div className="flex items-center gap-2">
                  <span className="text-sm text-content-secondary">Editing:</span>
                  <span className="text-sm font-medium text-content">{selectedTemplate}.conf</span>
                  {hasChanges && (
                    <span className="text-xs px-1.5 py-0.5 bg-amber-500/20 text-amber-400 rounded">
                      Unsaved
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleReset}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-surface-raised hover:bg-hover rounded-lg transition-colors"
                    title="Reset to default"
                  >
                    <RotateCcw className="w-4 h-4" />
                    Reset
                  </button>
                  {isCustom && (
                    <button
                      onClick={handleDelete}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg transition-colors"
                      title="Delete template"
                    >
                      <Trash2 className="w-4 h-4" />
                      Delete
                    </button>
                  )}
                  <button
                    onClick={handleSave}
                    disabled={saving || !hasChanges}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-purple-600 hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors"
                  >
                    <Save className="w-4 h-4" />
                    {saving ? 'Saving...' : 'Save'}
                  </button>
                </div>
              </div>

              {/* Editor Area */}
              <div className="flex-1 overflow-hidden">
                <textarea
                  value={content}
                  onChange={(e) => setContent(e.target.value)}
                  className="w-full h-full p-4 bg-surface text-content font-mono text-sm resize-none focus:outline-none"
                  spellCheck={false}
                  placeholder="Template content..."
                />
              </div>

              {/* Variable Reference */}
              <div className="p-3 border-t border-edge bg-surface">
                <div className="text-xs text-content-muted">
                  <span className="font-medium text-content-secondary">Available variables:</span>{' '}
                  <code className="text-purple-400">{'{{domain}}'}</code>,{' '}
                  <code className="text-purple-400">{'{{port}}'}</code>,{' '}
                  <code className="text-purple-400">{'{{path}}'}</code>,{' '}
                  <code className="text-purple-400">{'{{php_port}}'}</code>,{' '}
                  <code className="text-purple-400">{'{{ssl_port}}'}</code>,{' '}
                  <code className="text-purple-400">{'{{ssl_cert}}'}</code>,{' '}
                  <code className="text-purple-400">{'{{ssl_key}}'}</code>
                </div>
              </div>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center text-content-muted">
              Select a template to edit
            </div>
          )}
        </div>
      </div>

      {/* New Template Dialog */}
      {showNewDialog && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-surface rounded-xl border border-edge p-6 w-96">
            <h3 className="text-lg font-semibold text-content mb-4">Create New Template</h3>
            <input
              type="text"
              value={newTemplateName}
              onChange={(e) => setNewTemplateName(e.target.value)}
              placeholder="Template name (e.g., my-custom-site)"
              className="w-full px-3 py-2 bg-surface-raised border border-edge rounded-lg focus:outline-none focus:border-purple-500"
              onKeyDown={(e) => e.key === 'Enter' && handleCreateNew()}
            />
            <p className="text-xs text-content-muted mt-2">
              Use lowercase letters, numbers, and hyphens only
            </p>
            <div className="flex justify-end gap-2 mt-4">
              <button
                onClick={() => {
                  setShowNewDialog(false);
                  setNewTemplateName('');
                }}
                className="px-4 py-2 text-sm bg-surface-raised hover:bg-hover rounded-lg transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateNew}
                disabled={!newTemplateName.trim()}
                className="px-4 py-2 text-sm bg-purple-600 hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
