import { useState, useEffect } from 'react'
import {
  getMcpStatus,
  installMcp,
  uninstallMcp,
  startMcp,
  stopMcp,
  checkMcpUpdate,
  updateMcp,
  McpStatus,
  BinaryUpdateInfo
} from '../lib/api'
import {
  Download,
  Trash2,
  Play,
  Square,
  RefreshCw,
  Sparkles,
  Copy,
  Check,
  ArrowUpCircle
} from 'lucide-react'
import { useApp } from '../lib/AppContext'

const CONFIG_SNIPPET = JSON.stringify(
  { mcpServers: { orbit: { command: 'orbit-mcp' } } },
  null,
  2
)

const CONFIG_PATHS = [
  { label: 'Claude Code', file: '~/.claude.json' },
  { label: 'Cursor', file: '.cursor/mcp.json' },
  { label: 'Windsurf', file: '~/.codeium/windsurf/mcp_config.json' },
]

export function McpManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<McpStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const [updateInfo, setUpdateInfo] = useState<BinaryUpdateInfo | null>(null)
  const [showConfig, setShowConfig] = useState(false)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getMcpStatus()
      setStatus(result)
      if (result.installed) {
        checkMcpUpdate().then(setUpdateInfo).catch(() => { })
      }
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load MCP status' })
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  const reloadStatusOnly = async () => {
    try {
      const result = await getMcpStatus()
      setStatus(result)
    } catch { /* ignore */ }
  }

  useEffect(() => {
    loadStatus()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const wrap = async (action: string, fn: () => Promise<unknown>, msg: string, onDone?: () => void) => {
    try {
      setActionLoading(action)
      await fn()
      addToast({ type: 'success', message: msg })
      onDone?.()
      await (action === 'update' ? reloadStatusOnly() : loadStatus())
    } catch {
      addToast({ type: 'error', message: `Failed to ${action} MCP server` })
    } finally {
      setActionLoading(null)
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(CONFIG_SNIPPET)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      addToast({ type: 'error', message: 'Failed to copy to clipboard' })
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <RefreshCw className="w-5 h-5 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
      {/* Header row — compact */}
      <div className="flex items-center gap-3 px-4 py-3">
        <Sparkles size={16} className="text-purple-500 shrink-0" />
        <span className="text-sm font-medium w-28 shrink-0">MCP Server</span>

        {/* Status */}
        {!status?.installed ? (
          <span className="text-[11px] text-content-muted">Not installed</span>
        ) : (
          <div className="flex items-center gap-2 text-[11px]">
            <span className={`px-1.5 py-0.5 rounded ${status.running ? 'bg-emerald-500/15 text-emerald-400' : 'bg-yellow-500/15 text-yellow-400'}`}>
              {status.running ? 'Running' : 'Stopped'}
            </span>
            {updateInfo?.has_update && (
              <span className="px-1.5 py-0.5 bg-amber-500/15 text-amber-400 rounded">
                v{updateInfo.latest_version}
              </span>
            )}
          </div>
        )}

        <div className="flex-1" />

        {/* Actions */}
        <div className="flex items-center gap-1">
          {!status?.installed ? (
            <IconBtn
              title="Install"
              disabled={actionLoading !== null}
              spinning={actionLoading === 'install'}
              onClick={() => wrap('install', installMcp, 'MCP server installed')}
              className="text-emerald-500 hover:bg-emerald-500/10"
            >
              <Download size={14} />
            </IconBtn>
          ) : (
            <>
              {/* Start/Stop */}
              {status.running ? (
                <IconBtn
                  title="Stop"
                  disabled={actionLoading !== null}
                  spinning={actionLoading === 'stop'}
                  onClick={() => wrap('stop', stopMcp, 'MCP server stopped')}
                  className="text-red-500/60 hover:bg-red-500/10 hover:text-red-500"
                >
                  <Square size={14} />
                </IconBtn>
              ) : (
                <IconBtn
                  title="Start"
                  disabled={actionLoading !== null}
                  spinning={actionLoading === 'start'}
                  onClick={() => wrap('start', startMcp, 'MCP server started')}
                  className="text-emerald-500 hover:bg-emerald-500/10"
                >
                  <Play size={14} />
                </IconBtn>
              )}
              {/* Update */}
              {updateInfo?.has_update && (
                <IconBtn
                  title={`Update to v${updateInfo.latest_version}`}
                  disabled={actionLoading !== null}
                  spinning={actionLoading === 'update'}
                  onClick={() => wrap('update', updateMcp, 'MCP server updated', () => setUpdateInfo(null))}
                  className="text-amber-500 hover:bg-amber-500/10"
                >
                  <ArrowUpCircle size={14} />
                </IconBtn>
              )}
              {/* Config toggle */}
              <IconBtn
                title={showConfig ? 'Hide config' : 'Show config'}
                disabled={false}
                spinning={false}
                onClick={() => setShowConfig(!showConfig)}
                className={showConfig ? 'text-purple-400 bg-purple-500/10' : 'text-content-muted hover:bg-hover'}
              >
                <Copy size={14} />
              </IconBtn>
              {/* Uninstall */}
              <IconBtn
                title="Uninstall"
                disabled={actionLoading !== null}
                spinning={actionLoading === 'uninstall'}
                onClick={() => wrap('uninstall', async () => { await stopMcp().catch(() => {}); await uninstallMcp(); }, 'MCP server uninstalled', () => setUpdateInfo(null))}
                className="text-red-500/60 hover:bg-red-500/10 hover:text-red-500"
              >
                <Trash2 size={14} />
              </IconBtn>
            </>
          )}
        </div>
      </div>

      {/* Config panel — collapsible */}
      {showConfig && status?.installed && (
        <div className="px-4 pb-3 border-t border-edge-subtle">
          <div className="mt-3 relative">
            <pre className="p-3 bg-surface-inset rounded-lg text-[11px] font-mono overflow-x-auto border border-edge-subtle">
              {CONFIG_SNIPPET}
            </pre>
            <button
              onClick={handleCopy}
              className="absolute top-2 right-2 p-1 bg-surface hover:bg-hover border border-edge rounded transition-colors"
              title="Copy"
            >
              {copied ? <Check size={12} className="text-emerald-400" /> : <Copy size={12} className="text-content-muted" />}
            </button>
          </div>
          <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1">
            {CONFIG_PATHS.map(({ label, file }) => (
              <span key={label} className="text-[10px] text-content-muted">
                <span className="text-content-secondary">{label}:</span> <code className="font-mono">{file}</code>
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function IconBtn({
  title,
  disabled,
  spinning,
  onClick,
  className,
  children,
}: {
  title: string
  disabled: boolean
  spinning: boolean
  onClick: () => void
  className: string
  children: React.ReactNode
}) {
  return (
    <button
      title={title}
      disabled={disabled}
      onClick={onClick}
      className={`p-1.5 rounded-lg transition-colors disabled:opacity-40 cursor-pointer ${className}`}
    >
      {spinning ? <RefreshCw size={14} className="animate-spin" /> : children}
    </button>
  )
}
