import { useState, useEffect } from 'react'
import {
  getComposerStatus,
  installComposer,
  uninstallComposer,
  updateComposer,
  ComposerStatus
} from '../lib/api'
import {
  Download,
  Trash2,
  RefreshCw,
  Package
} from 'lucide-react'
import { open } from '@tauri-apps/plugin-shell'
import { useApp } from '../lib/AppContext'
import { InfoTooltip } from './InfoTooltip'

export function ComposerManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<ComposerStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getComposerStatus()
      setStatus(result)
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load Composer status' })
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadStatus()
  }, [])

  const handleInstall = async () => {
    try {
      setActionLoading('install')
      await installComposer()
      addToast({ type: 'success', message: 'Composer installed successfully' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to install Composer' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUninstall = async () => {
    try {
      setActionLoading('uninstall')
      await uninstallComposer()
      addToast({ type: 'success', message: 'Composer uninstalled' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to uninstall Composer' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUpdate = async () => {
    try {
      setActionLoading('update')
      await updateComposer()
      addToast({ type: 'success', message: 'Composer updated successfully' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to update Composer' })
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
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-6">
      <div className="flex items-center gap-3 mb-4">
        <div className="p-3 bg-orange-500/10 rounded-xl">
          <Package className="w-6 h-6 text-orange-500" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">Composer</h2>
          <p className="text-content-secondary text-sm">PHP dependency manager</p>
        </div>
        <InfoTooltip
          content={
            <div className="space-y-2 text-content-secondary">
              <p>PHP dependency manager for libraries and packages.</p>
              <div className="space-y-1 font-mono text-xs text-content-muted">
                <p>composer install</p>
                <p>composer update</p>
                <p>composer require vendor/pkg</p>
              </div>
              <button
                onClick={() => open('https://getcomposer.org/doc/')}
                className="text-emerald-500 hover:text-emerald-400 text-xs transition-colors"
              >
                getcomposer.org/doc
              </button>
            </div>
          }
        />
        <div className="ml-auto">
          {status?.installed ? (
            <span className="px-2 py-1 bg-emerald-500/20 text-emerald-400 rounded text-xs font-medium">
              Installed
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
            <span className="text-content-muted">Version:</span>
            <span className="font-mono">{status.version || 'Unknown'}</span>
          </div>
          <div className="flex justify-between mb-1">
            <span className="text-content-muted">PHP:</span>
            <span className="font-mono">{status.php_version || 'Unknown'}</span>
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
            <button
              onClick={handleUpdate}
              disabled={actionLoading !== null}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
            >
              {actionLoading === 'update' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <RefreshCw size={16} />
              )}
              Update
            </button>
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
