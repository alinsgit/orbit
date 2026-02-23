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
import { InfoTooltip } from './InfoTooltip'

type ConfigTab = 'claude' | 'cursor' | 'windsurf'

export function McpManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<McpStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)
  const [activeConfig, setActiveConfig] = useState<ConfigTab>('claude')
  const [copied, setCopied] = useState(false)
  const [updateInfo, setUpdateInfo] = useState<BinaryUpdateInfo | null>(null)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getMcpStatus()
      setStatus(result)
      // Check for updates if installed
      if (result.installed) {
        checkMcpUpdate().then(setUpdateInfo).catch(() => {})
      }
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load MCP status' })
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
      await installMcp()
      addToast({ type: 'success', message: 'MCP server installed successfully' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to install MCP server' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUninstall = async () => {
    try {
      setActionLoading('uninstall')
      await stopMcp().catch(() => {})
      await uninstallMcp()
      addToast({ type: 'success', message: 'MCP server uninstalled' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to uninstall MCP server' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStart = async () => {
    try {
      setActionLoading('start')
      await startMcp()
      addToast({ type: 'success', message: 'MCP server started' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to start MCP server' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleStop = async () => {
    try {
      setActionLoading('stop')
      await stopMcp()
      addToast({ type: 'success', message: 'MCP server stopped' })
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to stop MCP server' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUpdate = async () => {
    try {
      setActionLoading('update')
      await updateMcp()
      addToast({ type: 'success', message: 'MCP server updated successfully' })
      setUpdateInfo(null)
      await loadStatus()
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to update MCP server' })
    } finally {
      setActionLoading(null)
    }
  }

  const getConfigSnippet = (): string => {
    const serverConfig = {
      command: "orbit-mcp",
      args: [] as string[]
    }

    switch (activeConfig) {
      case 'claude':
        return JSON.stringify({
          mcpServers: {
            orbit: serverConfig
          }
        }, null, 2)
      case 'cursor':
        return JSON.stringify({
          mcpServers: {
            orbit: serverConfig
          }
        }, null, 2)
      case 'windsurf':
        return JSON.stringify({
          mcpServers: {
            orbit: serverConfig
          }
        }, null, 2)
    }
  }

  const getConfigInfo = (): { file: string; description: string } => {
    switch (activeConfig) {
      case 'claude':
        return {
          file: '~/.claude.json',
          description: 'Add to your global Claude Code config'
        }
      case 'cursor':
        return {
          file: '.cursor/mcp.json',
          description: 'Add to your project .cursor directory'
        }
      case 'windsurf':
        return {
          file: '~/.codeium/windsurf/mcp_config.json',
          description: 'Add to your Windsurf global config'
        }
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(getConfigSnippet())
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      addToast({ type: 'error', message: 'Failed to copy to clipboard' })
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <RefreshCw className="w-8 h-8 animate-spin text-emerald-500" />
      </div>
    )
  }

  const configTabs: { key: ConfigTab; label: string }[] = [
    { key: 'claude', label: 'Claude Code' },
    { key: 'cursor', label: 'Cursor' },
    { key: 'windsurf', label: 'Windsurf' },
  ]

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-6">
      <div className="flex items-center gap-3 mb-4">
        <div className="p-3 bg-purple-500/10 rounded-xl">
          <Sparkles className="w-6 h-6 text-purple-500" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">MCP Server</h2>
          <p className="text-content-secondary text-sm">AI tool integration</p>
        </div>
        <InfoTooltip
          content={
            <div className="space-y-2 text-content-secondary">
              <p>Model Context Protocol (MCP) allows AI tools like Claude Code, Cursor, and Windsurf to manage your Orbit services, sites, and databases.</p>
              <p className="text-xs text-content-muted">Install the MCP server, then add the config snippet to your AI tool.</p>
            </div>
          }
        />
        <div className="ml-auto flex items-center gap-2">
          {updateInfo?.has_update && (
            <span className="px-2 py-1 bg-amber-500/20 text-amber-400 rounded text-xs font-medium">
              Update Available (v{updateInfo.latest_version})
            </span>
          )}
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
        <>
          {/* Path info */}
          <div className="mb-4 p-3 bg-surface-inset rounded-lg text-sm">
            <div className="flex justify-between">
              <span className="text-content-muted">Path:</span>
              <span className="font-mono text-xs truncate max-w-[300px]">{status.path}</span>
            </div>
          </div>

          {/* Config snippets */}
          <div className="mb-4">
            <p className="text-sm font-medium mb-2">Configuration</p>
            <div className="flex gap-1 mb-2">
              {configTabs.map(tab => (
                <button
                  key={tab.key}
                  onClick={() => { setActiveConfig(tab.key); setCopied(false) }}
                  className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${activeConfig === tab.key
                    ? 'bg-purple-500/20 text-purple-400'
                    : 'text-content-muted hover:text-content-secondary hover:bg-hover'
                  }`}
                >
                  {tab.label}
                </button>
              ))}
            </div>
            <div className="relative">
              <pre className="p-3 bg-surface-inset rounded-lg text-xs font-mono overflow-x-auto border border-edge-subtle">
                {getConfigSnippet()}
              </pre>
              <button
                onClick={handleCopy}
                className="absolute top-2 right-2 p-1.5 bg-surface hover:bg-hover border border-edge rounded-md transition-colors"
                title="Copy to clipboard"
              >
                {copied ? <Check size={14} className="text-emerald-400" /> : <Copy size={14} className="text-content-muted" />}
              </button>
            </div>
            <p className="text-xs text-content-muted mt-1.5">
              {getConfigInfo().description} ({getConfigInfo().file})
            </p>
          </div>
        </>
      )}

      {/* Action buttons */}
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
            {updateInfo?.has_update && (
              <button
                onClick={handleUpdate}
                disabled={actionLoading !== null}
                className="flex items-center gap-2 px-4 py-2 bg-amber-600/20 hover:bg-amber-600/30 text-amber-400 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
              >
                {actionLoading === 'update' ? (
                  <RefreshCw size={16} className="animate-spin" />
                ) : (
                  <ArrowUpCircle size={16} />
                )}
                Update
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
