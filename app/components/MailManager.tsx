import { useState, useEffect } from 'react'
import {
  getMailpitStatus,
  installMailpit,
  uninstallMailpit,
  startMailpit,
  stopMailpit,
  MailpitStatus
} from '../lib/api'
import {
  Download,
  Trash2,
  Play,
  Square,
  RefreshCw,
  Mail,
  ExternalLink
} from 'lucide-react'
import { open } from '@tauri-apps/plugin-shell'
import { useApp } from '../lib/AppContext'

export function MailManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<MailpitStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getMailpitStatus()
      setStatus(result)
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load Mailpit status' })
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
      await installMailpit()
      addToast({ type: 'success', message: 'Mailpit installed successfully' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to install Mailpit' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUninstall = async () => {
    try {
      setActionLoading('uninstall')
      await stopMailpit().catch(() => {})
      await uninstallMailpit()
      addToast({ type: 'success', message: 'Mailpit uninstalled' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to uninstall Mailpit' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStart = async () => {
    try {
      setActionLoading('start')
      await startMailpit()
      addToast({ type: 'success', message: 'Mailpit started' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to start Mailpit' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStop = async () => {
    try {
      setActionLoading('stop')
      await stopMailpit()
      addToast({ type: 'success', message: 'Mailpit stopped' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to stop Mailpit' })
    } finally {
      setActionLoading(null)
    }
  }

  const openWebUI = () => {
    open(`http://localhost:${status?.web_port || 8025}`)
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
        <div className="p-2 bg-pink-500/10 rounded-lg">
          <Mail className="w-4 h-4 text-pink-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold">Mailpit</span>
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
            Email testing{status?.installed ? ` · SMTP :${status.smtp_port} · UI :${status.web_port}` : ''}
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
                  <button onClick={openWebUI} className="p-1.5 text-pink-400 hover:bg-pink-500/15 rounded-lg transition-colors" title="Open Web UI">
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
