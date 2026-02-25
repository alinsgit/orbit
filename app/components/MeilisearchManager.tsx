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
      await stopMeilisearch().catch(() => { })
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
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="w-8 h-8 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-6">
      <div className="flex items-center gap-3 mb-4">
        <div className="p-3 bg-purple-500/10 rounded-xl">
          <Search className="w-6 h-6 text-purple-500" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Meilisearch</h2>
          <p className="text-content-secondary text-sm">Fast, typo-tolerant search engine</p>
        </div>
        <div className="ml-auto">
          {status?.installed ? (
            <span className={`px-2 py-1 rounded text-xs font-medium ${status.running
              ? 'bg-emerald-500/20 text-emerald-400'
              : 'bg-yellow-500/20 text-yellow-400'
              }`}>
              {status.running ? 'Running' : 'Stopped'}
            </span>
          ) : (
            <span className="px-2 py-1 bg-gray-500/20 text-gray-400 rounded text-xs font-medium">
              Not Installed
            </span>
          )}
        </div>
      </div>

      {status?.installed && (
        <div className="mb-4 p-3 bg-surface-inset rounded-lg text-sm">
          <div className="flex justify-between mb-1">
            <span className="text-content-muted">HTTP Port:</span>
            <span className="font-mono">{status.http_port || 7700}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-content-muted">Path:</span>
            <span className="font-mono text-xs truncate max-w-[200px]">{status.path}</span>
          </div>
        </div>
      )}

      <div className="flex gap-2">
        {!status?.installed ? (
          <button
            onClick={handleInstall}
            disabled={actionLoading !== null}
            className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          >
            {actionLoading === 'install' ? (
              <RefreshCw size={16} className="animate-spin" />
            ) : (
              <Download size={16} />
            )}
            Install
          </button>
        ) : (
          <>
            {status.running ? (
              <>
                <button
                  onClick={openWebUI}
                  className="flex items-center gap-2 px-4 py-2 bg-purple-600 hover:bg-purple-500 rounded-lg text-sm font-medium transition-colors"
                >
                  <ExternalLink size={16} />
                  Open Web UI
                </button>
                <button
                  onClick={handleStop}
                  disabled={actionLoading !== null}
                  className="flex items-center gap-2 px-4 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
                >
                  {actionLoading === 'stop' ? (
                    <RefreshCw size={16} className="animate-spin" />
                  ) : (
                    <Square size={16} />
                  )}
                  Stop
                </button>
              </>
            ) : (
              <button
                onClick={handleStart}
                disabled={actionLoading !== null}
                className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
              >
                {actionLoading === 'start' ? (
                  <RefreshCw size={16} className="animate-spin" />
                ) : (
                  <Play size={16} />
                )}
                Start
              </button>
            )}
            <button
              onClick={handleUninstall}
              disabled={actionLoading !== null}
              className="p-2 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg transition-colors disabled:opacity-50"
              title="Uninstall"
            >
              {actionLoading === 'uninstall' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <Trash2 size={16} />
              )}
            </button>
          </>
        )}
      </div>
    </div>
  )
}
