import { useState, useEffect } from 'react';
import { Save, X, Terminal, Loader2, AlertTriangle } from 'lucide-react';
import { getUserPath, saveUserPath } from '../lib/api';
import { useApp } from '../lib/AppContext';

interface PathEditorModalProps {
  onClose: () => void;
}

export function PathEditorModal({ onClose }: PathEditorModalProps) {
  const { addToast } = useApp();
  const [paths, setPaths] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [newPathInput, setNewPathInput] = useState('');

  // Initial load
  useEffect(() => {
    fetchPaths();
  }, []);

  const fetchPaths = async () => {
    try {
      setLoading(true);
      const data = await getUserPath();
      setPaths(data || []);
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to load PATH: ${e}` });
      onClose();
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    try {
      setSaving(true);
      const res = await saveUserPath(paths);
      addToast({ type: 'success', message: res });
      onClose();
    } catch (e: any) {
      addToast({ type: 'error', message: `Failed to save PATH: ${e}` });
    } finally {
      setSaving(false);
    }
  };

  const handleRemoveIndex = (idx: number) => {
    const updated = [...paths];
    updated.splice(idx, 1);
    setPaths(updated);
  };

  const handleAddPath = () => {
    const trimmed = newPathInput.trim();
    if (!trimmed) return;
    
    if (paths.includes(trimmed)) {
      addToast({ type: 'warning', message: 'Path already exists in list.' });
      return;
    }

    setPaths([trimmed, ...paths]);
    setNewPathInput('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddPath();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm animate-in fade-in duration-200">
      <div 
        className="bg-surface-alt border border-edge shadow-2xl rounded-xl w-full max-w-2xl flex flex-col h-[80vh] animate-in zoom-in-95 duration-200 overflow-hidden"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-edge bg-surface">
          <div>
            <h3 className="font-semibold text-lg flex items-center gap-2">
              <Terminal size={20} className="text-emerald-500" />
              Environment Variables (User PATH)
            </h3>
            <p className="text-sm text-content-secondary mt-1">
              Add or remove toolchain directories mapped to your OS.
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
          <div className="bg-amber-500/10 border border-amber-500/20 text-amber-500 p-3 rounded-lg flex gap-3 text-sm">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div>
              <p className="font-medium">Proceed with caution</p>
              <p className="opacity-90">Removing essential OS paths may break external commands.</p>
            </div>
          </div>

          <div className="flex gap-2">
            <input 
              type="text" 
              value={newPathInput} 
              onChange={e => setNewPathInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="E.g. C:\Users\YourName\AppData\Local\Programs\Python..."
              className="flex-1 px-4 py-2 border border-edge bg-surface-inset rounded-lg text-sm text-content-secondary outline-none focus:border-emerald-500"
            />
            <button 
              onClick={handleAddPath}
              className="px-4 py-2 bg-emerald-600/20 text-emerald-500 hover:bg-emerald-600/30 rounded-lg text-sm font-medium transition-colors"
            >
              Add Path
            </button>
          </div>
          
          <div className="border border-edge rounded-lg flex-1 min-h-0 overflow-hidden flex flex-col bg-surface-inset">
            {loading ? (
              <div className="p-8 flex items-center justify-center text-content-muted gap-2">
                <Loader2 size={18} className="animate-spin" /> Loading PATH...
              </div>
            ) : paths.length === 0 ? (
              <div className="p-8 text-center text-content-muted">Path is completely empty.</div>
            ) : (
              <div className="flex-1 overflow-auto">
                <table className="w-full text-left text-sm whitespace-nowrap">
                  <tbody>
                    {paths.map((p, idx) => (
                      <tr key={idx} className="border-b border-edge/50 hover:bg-hover/30 group">
                        <td className="p-3 pl-4 max-w-sm truncate text-content-secondary group-hover:text-content transition-colors">
                          {p}
                        </td>
                        <td className="p-3 w-16 text-right">
                          <button 
                            onClick={() => handleRemoveIndex(idx)}
                            className="p-1.5 text-red-500 opacity-0 group-hover:opacity-100 hover:bg-red-500/10 rounded transition-all"
                            title="Remove entry"
                          >
                            <X size={16} />
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
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
            Save Settings
          </button>
        </div>
      </div>
    </div>
  );
}
