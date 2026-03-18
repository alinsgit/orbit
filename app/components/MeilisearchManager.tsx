import { useState, useEffect } from 'react'
import {
  getMeilisearchStatus,
  installMeilisearch,
  uninstallMeilisearch,
  startMeilisearch,
  stopMeilisearch,
  MeilisearchStatus
} from '../lib/api'
import {
  Download,
  Trash2,
  Play,
  Square,
  RefreshCw,
  Search,
  ExternalLink
} from 'lucide-react'
import { open } from '@tauri-apps/plugin-shell'
import { useApp } from '../lib/AppContext'

export function MeilisearchManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<MeilisearchStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getMeilisearchStatus()
      setStatus(result)
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load Meilisearch status' })
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadStatus()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleInstall = async () => {
    try {
      setActionLoading('install')
      await installMeilisearch()
      addToast({ type: 'success', message: 'Meilisearch installed successfully' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to install Meilisearch' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUninstall = async () => {
    try {
      setActionLoading('uninstall')
      await stopMeilisearch().catch(() => {})
      await uninstallMeilisearch()
      addToast({ type: 'success', message: 'Meilisearch uninstalled' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to uninstall Meilisearch' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStart = async () => {
    try {
      setActionLoading('start')
      await startMeilisearch()
      addToast({ type: 'success', message: 'Meilisearch started' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to start Meilisearch' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStop = async () => {
    try {
      setActionLoading('stop')
      await stopMeilisearch()
      addToast({ type: 'success', message: 'Meilisearch stopped' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to stop Meilisearch' })
    } finally {
      setActionLoading(null)
    }
  }

  const openWebUI = () => {
    open(`http://localhost:${status?.http_port || 7700}`)
  }

  if (loading) {
    return (
      <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4 flex items-center justify-center h-[140px]">
        <RefreshCw className="w-5 h-5 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4">
      <div className="flex items-center gap-2.5">
        <div className="p-2 bg-purple-500/10 rounded-lg">
          <Search className="w-4 h-4 text-purple-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold">Meilisearch</span>
            {status?.installed ? (
              <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${
                status.running ? 'bg-emerald-500/15 text-emerald-400' : 'bg-yellow-500/15 text-yellow-400'
              }`}>
                {status.running ? 'Running' : 'Stopped'}
              </span>
            ) : (
              <span className="px-1.5 py-0.5 bg-zinc-500/15 text-zinc-400 rounded text-[10px] font-medium">
                Not Installed
              </span>
            )}
          </div>
          <p className="text-[11px] text-content-muted leading-tight">
            Search engine{status?.installed ? ` · HTTP :${status.http_port || 7700}` : ''}
          </p>
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {!status?.installed ? (
            <button onClick={handleInstall} disabled={actionLoading !== null} className="p-1.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg transition-colors disabled:opacity-50" title="Install">
              {actionLoading === 'install' ? <RefreshCw size={14} className="animate-spin" /> : <Download size={14} />}
            </button>
          ) : (
            <>
              {status.running ? (
                <>
                  <button onClick={openWebUI} className="p-1.5 text-purple-400 hover:bg-purple-500/15 rounded-lg transition-colors" title="Open Web UI">
                    <ExternalLink size={14} />
                  </button>
                  <button onClick={handleStop} disabled={actionLoading !== null} className="p-1.5 text-amber-400 hover:bg-amber-500/15 rounded-lg transition-colors disabled:opacity-50" title="Stop">
                    {actionLoading === 'stop' ? <RefreshCw size={14} className="animate-spin" /> : <Square size={14} />}
                  </button>
                </>
              ) : (
                <button onClick={handleStart} disabled={actionLoading !== null} className="p-1.5 text-emerald-400 hover:bg-emerald-500/15 rounded-lg transition-colors disabled:opacity-50" title="Start">
                  {actionLoading === 'start' ? <RefreshCw size={14} className="animate-spin" /> : <Play size={14} />}
                </button>
              )}
              <button onClick={handleUninstall} disabled={actionLoading !== null} className="p-1.5 text-red-400 hover:bg-red-500/15 rounded-lg transition-colors disabled:opacity-50" title="Uninstall">
                {actionLoading === 'uninstall' ? <RefreshCw size={14} className="animate-spin" /> : <Trash2 size={14} />}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
