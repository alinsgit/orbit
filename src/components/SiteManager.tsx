import { useState, useEffect } from 'react';
import { Plus, Trash2, FolderOpen, Globe, ExternalLink, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import { useApp } from '../lib/AppContext';
// import { open as openDialog } from '@tauri-apps/plugin-dialog'; // Need to add this plugin later

interface Site {
  domain: string;
  path: string;
  port: number;
  php_version?: string;
}

export function SiteManager() {
  const { addToast } = useApp();
  const [sites, setSites] = useState<Site[]>([]);
  const [loading, setLoading] = useState(false);
  const [showAddModal, setShowAddModal] = useState(false);
  
  // New Site Form State
  const [newSite, setNewSite] = useState<Site>({
    domain: '',
    path: '',
    port: 80,
  });

  const fetchSites = async () => {
    setLoading(true);
    try {
      const res = await invoke<Site[]>('get_sites');
      setSites(res);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSites();
  }, []);

  const handleCreateSite = async () => {
    if (!newSite.domain || !newSite.path) return;
    
    try {
      await invoke('create_site', { site: newSite });
      setShowAddModal(false);
      setNewSite({ domain: '', path: '', port: 80 });
      fetchSites();
    } catch (e) {
      addToast({ type: 'error', message: 'Failed to create site: ' + e });
    }
  };

  const openInBrowser = (domain: string) => {
    open(`http://${domain}`);
  };

  return (
    <div className="p-6 h-full flex flex-col">
      <header className="flex justify-between items-center mb-6">
        <div>
          <h2 className="text-2xl font-bold">Sites</h2>
          <p className="text-neutral-400">Manage your local virtual hosts</p>
        </div>
        <button
          onClick={() => setShowAddModal(true)}
          className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg flex items-center gap-2 transition-colors text-sm font-medium"
        >
          <Plus size={16} />
          New Site
        </button>
      </header>

      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex justify-center py-12">
            <Loader2 className="animate-spin text-emerald-500" />
          </div>
        ) : sites.length === 0 ? (
          <div className="text-center py-12 text-neutral-500 border-2 border-dashed border-neutral-800 rounded-xl">
            <Globe className="mx-auto mb-4 opacity-50" size={48} />
            <p>No sites configured.</p>
            <p className="text-sm mt-2">Click "New Site" to create your first local domain.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {sites.map((site) => (
              <div key={site.domain} className="bg-neutral-800/50 border border-neutral-800 p-4 rounded-xl hover:border-neutral-700 transition-all group">
                <div className="flex justify-between items-start mb-3">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-lg bg-emerald-500/10 text-emerald-500 flex items-center justify-center">
                      <Globe size={20} />
                    </div>
                    <div>
                      <h3 className="font-semibold text-lg">{site.domain}</h3>
                      <div className="flex items-center gap-2 text-xs text-neutral-400">
                        <span className="bg-neutral-800 px-1.5 py-0.5 rounded">Port {site.port}</span>
                        {site.php_version && <span className="bg-neutral-800 px-1.5 py-0.5 rounded">PHP {site.php_version}</span>}
                      </div>
                    </div>
                  </div>
                  <div className="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button 
                      onClick={() => openInBrowser(site.domain)}
                      className="p-2 hover:bg-neutral-700 rounded-lg text-neutral-400 hover:text-white" title="Open in Browser">
                      <ExternalLink size={16} />
                    </button>
                    <button className="p-2 hover:bg-red-500/10 hover:text-red-500 rounded-lg text-neutral-400 transition-colors" title="Delete Site">
                      <Trash2 size={16} />
                    </button>
                  </div>
                </div>
                
                <div className="bg-neutral-900/50 p-2 rounded-lg text-xs font-mono text-neutral-500 truncate flex items-center gap-2">
                  <FolderOpen size={12} />
                  {site.path}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Add Site Modal */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-md shadow-2xl overflow-hidden">
            <div className="p-6">
              <h3 className="text-xl font-bold mb-4">Add New Site</h3>
              
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-neutral-400 mb-1">Domain Name</label>
                  <input
                    type="text"
                    placeholder="mysite.local"
                    value={newSite.domain}
                    onChange={(e) => setNewSite({...newSite, domain: e.target.value})}
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-2 focus:outline-none focus:border-emerald-500 transition-colors"
                  />
                  <p className="text-xs text-neutral-600 mt-1">Will be added to hosts file automatically.</p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-neutral-400 mb-1">Root Directory</label>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      placeholder="C:\Projects\MySite\public"
                      value={newSite.path}
                      onChange={(e) => setNewSite({...newSite, path: e.target.value})}
                      className="flex-1 bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-2 focus:outline-none focus:border-emerald-500 transition-colors font-mono text-sm"
                    />
                    {/* TODO: Implement folder picker */}
                    <button className="px-3 bg-neutral-800 hover:bg-neutral-700 rounded-lg transition-colors">
                      <FolderOpen size={18} />
                    </button>
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-neutral-400 mb-1">Port</label>
                  <input
                    type="number"
                    value={newSite.port}
                    onChange={(e) => setNewSite({...newSite, port: parseInt(e.target.value)})}
                    className="w-full bg-neutral-950 border border-neutral-800 rounded-lg px-4 py-2 focus:outline-none focus:border-emerald-500 transition-colors"
                  />
                </div>
              </div>

              <div className="flex gap-3 mt-8">
                <button
                  onClick={() => setShowAddModal(false)}
                  className="flex-1 px-4 py-2 bg-neutral-800 hover:bg-neutral-700 rounded-lg text-sm font-medium transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={handleCreateSite}
                  disabled={!newSite.domain || !newSite.path}
                  className="flex-1 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors"
                >
                  Create Site
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
