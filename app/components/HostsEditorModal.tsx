import { useState, useEffect } from 'react';
import { Save, X, Network, Loader2, FileWarning } from 'lucide-react';
import { getHostsFile, saveHostsFile } from '../lib/api';
import { useApp } from '../lib/AppContext';

interface HostsEditorModalProps {
  onClose: () => void;
}

export function HostsEditorModal({ onClose }: HostsEditorModalProps) {
  const { addToast } = useApp();
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    fetchHosts();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const fetchHosts = async () => {
    try {
      setLoading(true);
      const data = await getHostsFile();
      setContent(data);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to load hosts file: ${e}` });
      onClose();
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    try {
      setSaving(true);
      const res = await saveHostsFile(content);
      addToast({ type: 'success', message: res });
      onClose();
    } catch (e: any) {
      // Show elevated permission failures distinctly
      if (e.includes("Elevation was denied")) {
        addToast({ type: 'error', message: "Administrative privileges are required to save changes." });
      } else {
        addToast({ type: 'error', message: `Save error: ${e}` });
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm animate-in fade-in duration-200">
      <div 
        className="bg-surface-alt border border-edge shadow-2xl rounded-xl w-full max-w-3xl flex flex-col h-[85vh] animate-in zoom-in-95 duration-200 overflow-hidden"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-edge bg-surface">
          <div>
            <h3 className="font-semibold text-lg flex items-center gap-2">
              <Network size={20} className="text-emerald-500" />
              System Hosts Editor
            </h3>
            <p className="text-sm text-content-secondary mt-1">
              Manually map domain names to specific IP addresses across your OS.
            </p>
          </div>
          <button 
            onClick={onClose}
            className="p-2 hover:bg-hover rounded-lg transition-colors text-content-secondary hover:text-content"
          >
            <X size={20} />
          </button>
        </div>

        <div className="flex-1 overflow-auto p-6 flex flex-col gap-4">
          <div className="bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 p-3 flex gap-3 text-sm rounded-lg">
            <FileWarning size={18} className="shrink-0 mt-0.5" />
            <div>
              <p className="font-medium">Administrator Required</p>
              <p className="opacity-90">Saving changes to system hosts will trigger a UAC Prompt on Windows or a Polkit authentication request on Unix systems.</p>
            </div>
          </div>

          <div className="flex-1 min-h-0 border border-edge rounded-lg bg-surface-inset relative overflow-hidden flex flex-col">
            {loading ? (
              <div className="absolute inset-0 flex items-center justify-center bg-surface-inset text-content-muted gap-2 z-10">
                <Loader2 size={18} className="animate-spin" /> Loadings Hosts...
              </div>
            ) : null}
            <textarea
              value={content}
              onChange={e => setContent(e.target.value)}
              disabled={loading}
              className="w-full h-full p-4 bg-transparent resize-none outline-none font-mono text-sm leading-relaxed text-content-secondary focus:text-content transition-colors placeholder:text-content-muted"
              spellCheck={false}
              placeholder="# Example:\n127.0.0.1 test.local"
            />
          </div>
        </div>

        <div className="p-4 border-t border-edge bg-surface flex justify-end gap-3 mt-auto">
          <button 
            onClick={onClose}
            className="px-4 py-2 bg-surface-inset hover:bg-hover rounded-lg text-sm font-medium transition-colors"
          >
            Cancel
          </button>
          <button 
            onClick={handleSave}
            disabled={loading || saving}
            className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors"
          >
            {saving ? <Loader2 size={16} className="animate-spin" /> : <Save size={16} />}
            Save & Overwrite
          </button>
        </div>
      </div>
    </div>
  );
}
