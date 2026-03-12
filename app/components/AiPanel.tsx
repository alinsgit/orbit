import { useState } from 'react'
import {
  installClaudeCode,
  uninstallClaudeCode,
  installGeminiCli,
  uninstallGeminiCli,
  updateGeminiCli,
  checkGeminiUpdate,
  AiToolStatus,
} from '../lib/api'
import {
  Download,
  Trash2,
  RefreshCw,
  ExternalLink,
  Bot,
  Sparkles,
  AlertTriangle,
  ArrowUpCircle,
  Check,
} from 'lucide-react'
import { useApp } from '../lib/AppContext'
import { McpManager } from './McpManager'

export function AiPanel() {
  const {
    addToast,
    setActiveTab,
    services,
    claudeCodeStatus,
    geminiCliStatus,
    refreshAiToolStatus,
  } = useApp()

  const [claudeAction, setClaudeAction] = useState<string | null>(null)
  const [geminiAction, setGeminiAction] = useState<string | null>(null)
  const [geminiLatest, setGeminiLatest] = useState<string | null>(null)
  const [checkingUpdate, setCheckingUpdate] = useState(false)

  const nodeInstalled = services.some(s => s.service_type === 'nodejs')

  // Claude Code actions
  const handleClaudeInstall = async () => {
    try {
      setClaudeAction('install')
      await installClaudeCode()
      addToast({ type: 'success', message: 'Claude Code installed successfully' })
      await refreshAiToolStatus()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to install Claude Code: ${err}` })
    } finally {
      setClaudeAction(null)
    }
  }

  const handleClaudeUninstall = async () => {
    try {
      setClaudeAction('uninstall')
      await uninstallClaudeCode()
      addToast({ type: 'success', message: 'Claude Code uninstalled' })
      await refreshAiToolStatus()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to uninstall Claude Code: ${err}` })
    } finally {
      setClaudeAction(null)
    }
  }

  // Gemini CLI actions
  const handleGeminiInstall = async () => {
    try {
      setGeminiAction('install')
      await installGeminiCli()
      addToast({ type: 'success', message: 'Gemini CLI installed successfully' })
      await refreshAiToolStatus()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to install Gemini CLI: ${err}` })
    } finally {
      setGeminiAction(null)
    }
  }

  const handleGeminiUninstall = async () => {
    try {
      setGeminiAction('uninstall')
      await uninstallGeminiCli()
      addToast({ type: 'success', message: 'Gemini CLI uninstalled' })
      setGeminiLatest(null)
      await refreshAiToolStatus()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to uninstall Gemini CLI: ${err}` })
    } finally {
      setGeminiAction(null)
    }
  }

  const handleGeminiCheckUpdate = async () => {
    try {
      setCheckingUpdate(true)
      const latest = await checkGeminiUpdate()
      setGeminiLatest(latest)
      if (latest && geminiCliStatus?.version && latest === geminiCliStatus.version) {
        addToast({ type: 'info', message: 'Gemini CLI is up to date' })
      } else if (latest) {
        addToast({ type: 'info', message: `Update available: ${latest}` })
      }
    } catch (err) {
      addToast({ type: 'error', message: `Failed to check for updates: ${err}` })
    } finally {
      setCheckingUpdate(false)
    }
  }

  const handleGeminiUpdate = async () => {
    try {
      setGeminiAction('update')
      await updateGeminiCli()
      addToast({ type: 'success', message: 'Gemini CLI updated successfully' })
      setGeminiLatest(null)
      await refreshAiToolStatus()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to update Gemini CLI: ${err}` })
    } finally {
      setGeminiAction(null)
    }
  }

  const hasGeminiUpdate = geminiLatest && geminiCliStatus?.version && geminiLatest !== geminiCliStatus.version

  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Claude Code Card */}
        <ToolCard
          name="Claude Code"
          description="Anthropic's AI coding assistant"
          icon={<Bot className="w-6 h-6 text-orange-500" />}
          iconBg="bg-orange-500/10"
          status={claudeCodeStatus}
          autoUpdates
        >
          {!claudeCodeStatus?.installed ? (
            <button
              onClick={handleClaudeInstall}
              disabled={claudeAction !== null}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
            >
              {claudeAction === 'install' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <Download size={16} />
              )}
              Install
            </button>
          ) : (
            <>
              <button
                onClick={() => setActiveTab('claude-code')}
                disabled={claudeAction !== null}
                className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
              >
                <ExternalLink size={16} />
                Launch
              </button>
              <button
                onClick={handleClaudeUninstall}
                disabled={claudeAction !== null}
                className="p-2 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg transition-colors disabled:opacity-50"
                title="Uninstall"
              >
                {claudeAction === 'uninstall' ? (
                  <RefreshCw size={16} className="animate-spin" />
                ) : (
                  <Trash2 size={16} />
                )}
              </button>
            </>
          )}
        </ToolCard>

        {/* Gemini CLI Card */}
        <ToolCard
          name="Gemini CLI"
          description="Google's AI coding assistant"
          icon={<Sparkles className="w-6 h-6 text-blue-500" />}
          iconBg="bg-blue-500/10"
          status={geminiCliStatus}
          requiresNode={!nodeInstalled}
        >
          {!geminiCliStatus?.installed ? (
            <button
              onClick={handleGeminiInstall}
              disabled={geminiAction !== null || !nodeInstalled}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
            >
              {geminiAction === 'install' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <Download size={16} />
              )}
              Install
            </button>
          ) : geminiCliStatus?.source === 'system' ? (
            <>
              <button
                onClick={() => setActiveTab('gemini-cli')}
                disabled={geminiAction !== null}
                className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
              >
                <ExternalLink size={16} />
                Launch
              </button>
              <button
                onClick={handleGeminiInstall}
                disabled={geminiAction !== null || !nodeInstalled}
                className="flex items-center gap-2 px-3 py-2 bg-surface hover:bg-hover border border-edge rounded-lg text-xs font-medium transition-colors disabled:opacity-50 text-content-secondary"
                title="Install into Orbit for managed updates"
              >
                {geminiAction === 'install' ? (
                  <RefreshCw size={14} className="animate-spin" />
                ) : (
                  <Download size={14} />
                )}
                Install in Orbit
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => setActiveTab('gemini-cli')}
                disabled={geminiAction !== null}
                className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
              >
                <ExternalLink size={16} />
                Launch
              </button>
              {hasGeminiUpdate ? (
                <button
                  onClick={handleGeminiUpdate}
                  disabled={geminiAction !== null}
                  className="flex items-center gap-2 px-3 py-2 bg-amber-600 hover:bg-amber-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
                  title={`Update to ${geminiLatest}`}
                >
                  {geminiAction === 'update' ? (
                    <RefreshCw size={16} className="animate-spin" />
                  ) : (
                    <ArrowUpCircle size={16} />
                  )}
                  Update to {geminiLatest}
                </button>
              ) : (
                <button
                  onClick={handleGeminiCheckUpdate}
                  disabled={checkingUpdate || geminiAction !== null}
                  className="flex items-center gap-2 px-3 py-2 bg-surface hover:bg-hover border border-edge rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-content-secondary"
                >
                  {checkingUpdate ? (
                    <RefreshCw size={16} className="animate-spin" />
                  ) : (
                    <RefreshCw size={16} />
                  )}
                  Check Update
                </button>
              )}
              <button
                onClick={handleGeminiUninstall}
                disabled={geminiAction !== null}
                className="p-2 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg transition-colors disabled:opacity-50"
                title="Uninstall"
              >
                {geminiAction === 'uninstall' ? (
                  <RefreshCw size={16} className="animate-spin" />
                ) : (
                  <Trash2 size={16} />
                )}
              </button>
            </>
          )}
        </ToolCard>
      </div>

      {/* MCP Server Manager */}
      <McpManager />
    </div>
  )
}

// Shared card wrapper
function ToolCard({
  name,
  description,
  icon,
  iconBg,
  status,
  autoUpdates,
  requiresNode,
  children,
}: {
  name: string
  description: string
  icon: React.ReactNode
  iconBg: string
  status: AiToolStatus | null
  autoUpdates?: boolean
  requiresNode?: boolean
  children: React.ReactNode
}) {
  const sourceBadge = () => {
    if (!status?.installed) {
      return <span className="px-2 py-0.5 bg-gray-500/20 text-gray-400 rounded text-[11px] font-medium">Not Installed</span>
    }
    if (status.source === 'system') {
      return <span className="px-2 py-0.5 bg-blue-500/20 text-blue-400 rounded text-[11px] font-medium">System</span>
    }
    if (status.source === 'native') {
      return <span className="px-2 py-0.5 bg-emerald-500/20 text-emerald-400 rounded text-[11px] font-medium">Native</span>
    }
    return <span className="px-2 py-0.5 bg-emerald-500/20 text-emerald-400 rounded text-[11px] font-medium">Orbit</span>
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-5 flex flex-col">
      <div className="flex items-center gap-3 mb-3">
        <div className={`p-2.5 ${iconBg} rounded-xl`}>
          {icon}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h2 className="text-base font-semibold">{name}</h2>
            {sourceBadge()}
          </div>
          <p className="text-content-secondary text-xs">{description}</p>
        </div>
      </div>

      {requiresNode && (
        <div className="mb-3 p-2.5 bg-amber-500/10 border border-amber-500/20 rounded-lg flex items-center gap-2 text-xs text-amber-400">
          <AlertTriangle size={14} className="shrink-0" />
          <span>Node.js required. Install from Services tab.</span>
        </div>
      )}

      {status?.installed && (
        <div className="mb-3 p-2.5 bg-surface-inset rounded-lg text-xs space-y-1">
          <div className="flex justify-between">
            <span className="text-content-muted">Version</span>
            <span className="font-mono">{status.version || 'Unknown'}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-content-muted">Path</span>
            <span className="font-mono truncate max-w-[220px] text-content-secondary">{status.path}</span>
          </div>
          {autoUpdates && (
            <div className="flex justify-between">
              <span className="text-content-muted">Updates</span>
              <span className="flex items-center gap-1 text-emerald-400">
                <Check size={10} /> Auto
              </span>
            </div>
          )}
        </div>
      )}

      <div className="flex gap-2 mt-auto">
        {children}
      </div>
    </div>
  )
}
