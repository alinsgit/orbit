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
import { useApp } from '../lib/AppContext'

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleInstall = async () => {
    try {
      setActionLoading('install')
      await installComposer()
      addToast({ type: 'success', message: 'Composer installed successfully' })
      await loadStatus()
    } catch (_err) {
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
    } catch (_err) {
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
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to update Composer' })
    } finally {
      setActionLoading(null)
    }
  }

  if (loading) {
    return (
      <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4 flex items-center justify-center h-[140px]">
        <RefreshCw className="w-5 h-5 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4 flex flex-col">
      {/* Header */}
      <div className="flex items-center gap-2.5 mb-3">
        <div className="p-2 bg-orange-500/10 rounded-lg">
          <Package className="w-4 h-4 text-orange-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold">Composer</span>
            {status?.installed ? (
              <span className="px-1.5 py-0.5 bg-emerald-500/15 text-emerald-400 rounded text-[10px] font-medium">
                {status.version || 'Installed'}
              </span>
            ) : (
              <span className="px-1.5 py-0.5 bg-zinc-500/15 text-zinc-400 rounded text-[10px] font-medium">
                Not Installed
              </span>
            )}
          </div>
          <p className="text-[11px] text-content-muted leading-tight">PHP dependency manager</p>
        </div>
      </div>

      {/* Info */}
      {status?.installed && (
        <div className="text-[11px] text-content-muted mb-3 space-y-0.5">
          {status.php_version && <div>PHP: <span className="text-content-secondary font-mono">{status.php_version}</span></div>}
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-1.5 mt-auto">
        {!status?.installed ? (
          <button
            onClick={handleInstall}
            disabled={actionLoading !== null}
            className="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-xs font-medium transition-colors disabled:opacity-50 text-white"
          >
            {actionLoading === 'install' ? <RefreshCw size={13} className="animate-spin" /> : <Download size={13} />}
            Install
          </button>
        ) : (
          <>
            <button
              onClick={handleUpdate}
              disabled={actionLoading !== null}
              className="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 bg-blue-600/15 hover:bg-blue-600/25 text-blue-400 rounded-lg text-xs font-medium transition-colors disabled:opacity-50"
            >
              {actionLoading === 'update' ? <RefreshCw size={13} className="animate-spin" /> : <RefreshCw size={13} />}
              Update
            </button>
            <button
              onClick={handleUninstall}
              disabled={actionLoading !== null}
              className="p-1.5 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg transition-colors disabled:opacity-50"
              title="Uninstall"
            >
              {actionLoading === 'uninstall' ? <RefreshCw size={13} className="animate-spin" /> : <Trash2 size={13} />}
            </button>
          </>
        )}
      </div>
    </div>
  )
}
