import { useState, useEffect } from 'react'
import {
  getCacheStatus,
  installRedis,
  uninstallRedis,
  updateRedisConfig,
  startService,
  stopService,
  getRedisExePath,
  CacheStatus
} from '../lib/api'
import {
  Download,
  Trash2,
  Play,
  Square,
  Settings,
  RefreshCw,
  Database
} from 'lucide-react'
import { useApp } from '../lib/AppContext'
import { InfoTooltip } from './InfoTooltip'

export function CacheManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<CacheStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  // Config modal state
  const [configModal, setConfigModal] = useState<{ port: number; maxMemory: string } | null>(null)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getCacheStatus()
      setStatus(result)
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load cache status' })
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadStatus()
  }, [])

  const handleInstallRedis = async () => {
    try {
      setActionLoading('install-redis')
      await installRedis()
      addToast({ type: 'success', message: 'Redis installed successfully' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to install Redis' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUninstallRedis = async () => {
    try {
      setActionLoading('uninstall-redis')
      await stopService('redis')
      await uninstallRedis()
      addToast({ type: 'success', message: 'Redis uninstalled' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to uninstall Redis' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStartRedis = async () => {
    try {
      setActionLoading('start-redis')
      const exePath = await getRedisExePath()
      await startService('redis', exePath)
      addToast({ type: 'success', message: 'Redis started' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to start Redis' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStopRedis = async () => {
    try {
      setActionLoading('stop-redis')
      await stopService('redis')
      addToast({ type: 'success', message: 'Redis stopped' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to stop Redis' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUpdateConfig = async () => {
    if (!configModal) return
    try {
      setActionLoading('update-config')
      await updateRedisConfig(configModal.port, configModal.maxMemory)
      addToast({ type: 'success', message: 'Configuration updated' })
      setConfigModal(null)
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to update configuration' })
    } finally {
      setActionLoading(null)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="w-8 h-8 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div>
      {/* Redis Card */}
      <div className="bg-surface-raised border border-edge-subtle rounded-xl p-6">
        <div className="flex items-center gap-3 mb-4">
          <div className="p-3 bg-red-500/10 rounded-xl">
            <Database className="w-6 h-6 text-red-500" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">Redis</h2>
            <p className="text-content-secondary text-sm">In-memory data structure store</p>
          </div>
          <InfoTooltip
            content={
              <div className="space-y-1 text-content-secondary">
                <p>Redis is an in-memory data store used for caching.</p>
                <p className="font-mono text-xs text-content-muted">Default port: 6379</p>
              </div>
            }
          />
          <div className="ml-auto">
            {status?.redis_installed ? (
              <span className={`px-2 py-1 rounded text-xs font-medium ${status.redis_running
                ? 'bg-emerald-500/20 text-emerald-400'
                : 'bg-yellow-500/20 text-yellow-400'
                }`}>
                {status.redis_running ? 'Running' : 'Stopped'}
              </span>
            ) : (
              <span className="px-2 py-1 bg-gray-500/20 text-gray-400 rounded text-xs font-medium">
                Not Installed
              </span>
            )}
          </div>
        </div>

        {status?.redis_installed && (
          <div className="mb-4 p-3 bg-surface-inset rounded-lg text-sm">
            <div className="flex justify-between mb-1">
              <span className="text-content-muted">Port:</span>
              <span className="font-mono">{status.redis_port}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-content-muted">Path:</span>
              <span className="font-mono text-xs truncate max-w-[200px]">{status.redis_path}</span>
            </div>
          </div>
        )}

        <div className="flex gap-2">
          {!status?.redis_installed ? (
            <button
              onClick={handleInstallRedis}
              disabled={actionLoading !== null}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
            >
              {actionLoading === 'install-redis' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <Download size={16} />
              )}
              Install
            </button>
          ) : (
            <>
              {status.redis_running ? (
                <button
                  onClick={handleStopRedis}
                  disabled={actionLoading !== null}
                  className="flex items-center gap-2 px-4 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
                >
                  {actionLoading === 'stop-redis' ? (
                    <RefreshCw size={16} className="animate-spin" />
                  ) : (
                    <Square size={16} />
                  )}
                  Stop
                </button>
              ) : (
                <button
                  onClick={handleStartRedis}
                  disabled={actionLoading !== null}
                  className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
                >
                  {actionLoading === 'start-redis' ? (
                    <RefreshCw size={16} className="animate-spin" />
                  ) : (
                    <Play size={16} />
                  )}
                  Start
                </button>
              )}
              <button
                onClick={() => setConfigModal({
                  port: status.redis_port,
                  maxMemory: '128mb'
                })}
                className="p-2 bg-surface hover:bg-hover rounded-lg transition-colors"
                title="Configure"
              >
                <Settings size={16} />
              </button>
              <button
                onClick={handleUninstallRedis}
                disabled={actionLoading !== null}
                className="p-2 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg transition-colors disabled:opacity-50"
                title="Uninstall"
              >
                <Trash2 size={16} />
              </button>
            </>
          )}
        </div>
      </div>

      {/* Config Modal */}
      {configModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-surface-raised border border-edge rounded-xl p-6 w-full max-w-md">
            <h3 className="text-lg font-semibold mb-4">Configure Redis</h3>

            <div className="space-y-4">
              <div>
                <label className="block text-sm text-content-secondary mb-1">Port</label>
                <input
                  type="number"
                  value={configModal.port}
                  onChange={(e) => setConfigModal({ ...configModal, port: parseInt(e.target.value) })}
                  className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500"
                />
              </div>
              <div>
                <label className="block text-sm text-content-secondary mb-1">Max Memory</label>
                <input
                  type="text"
                  value={configModal.maxMemory}
                  onChange={(e) => setConfigModal({ ...configModal, maxMemory: e.target.value })}
                  placeholder="e.g., 128mb, 1gb"
                  className="w-full px-3 py-2 bg-surface-inset border border-edge rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500"
                />
              </div>
            </div>

            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => setConfigModal(null)}
                className="px-4 py-2 bg-surface hover:bg-hover rounded-lg text-sm transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleUpdateConfig}
                disabled={actionLoading === 'update-config'}
                className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
              >
                {actionLoading === 'update-config' && <RefreshCw size={14} className="animate-spin" />}
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
