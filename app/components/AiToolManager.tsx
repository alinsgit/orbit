import { useState, useEffect } from 'react'
import {
  getClaudeCodeStatus,
  installClaudeCode,
  uninstallClaudeCode,
  updateClaudeCode,
  getGeminiCliStatus,
  installGeminiCli,
  uninstallGeminiCli,
  updateGeminiCli,
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
} from 'lucide-react'
import { useApp } from '../lib/AppContext'
import { InfoTooltip } from './InfoTooltip'

interface AiToolCardProps {
  name: string
  description: string
  icon: React.ReactNode
  iconBg: string
  status: AiToolStatus | null
  loading: boolean
  actionLoading: string | null
  onInstall: () => void
  onUninstall: () => void
  onUpdate: () => void
  onLaunch: () => void
  tooltip: React.ReactNode
  nodeInstalled: boolean
}

function AiToolCard({
  name,
  description,
  icon,
  iconBg,
  status,
  loading,
  actionLoading,
  onInstall,
  onUninstall,
  onUpdate,
  onLaunch,
  tooltip,
  nodeInstalled,
}: AiToolCardProps) {
  if (loading) {
    return (
      <div className="bg-surface-raised border border-edge-subtle rounded-xl p-6 flex items-center justify-center py-12">
        <RefreshCw className="w-8 h-8 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-6">
      <div className="flex items-center gap-3 mb-4">
        <div className={`p-3 ${iconBg} rounded-xl`}>
          {icon}
        </div>
        <div>
          <h2 className="text-lg font-semibold">{name}</h2>
          <p className="text-content-secondary text-sm">{description}</p>
        </div>
        <InfoTooltip content={tooltip} />
        <div className="ml-auto">
          {status?.installed ? (
            status.source === 'system' ? (
              <span className="px-2 py-1 bg-blue-500/20 text-blue-400 rounded text-xs font-medium">
                System
              </span>
            ) : (
              <span className="px-2 py-1 bg-emerald-500/20 text-emerald-400 rounded text-xs font-medium">
                Installed
              </span>
            )
          ) : (
            <span className="px-2 py-1 bg-gray-500/20 text-gray-400 rounded text-xs font-medium">
              Not Installed
            </span>
          )}
        </div>
      </div>

      {!nodeInstalled && (
        <div className="mb-4 p-3 bg-amber-500/10 border border-amber-500/20 rounded-lg flex items-center gap-2 text-sm text-amber-400">
          <AlertTriangle size={16} className="shrink-0" />
          <span>Node.js is required. Install it from the Runtimes tab first.</span>
        </div>
      )}

      {status?.installed && (
        <div className="mb-4 p-3 bg-surface-inset rounded-lg text-sm">
          <div className="flex justify-between mb-1">
            <span className="text-content-muted">Version:</span>
            <span className="font-mono">{status.version || 'Unknown'}</span>
          </div>
          <div className="flex justify-between mb-1">
            <span className="text-content-muted">Source:</span>
            <span className={`font-mono text-xs ${status.source === 'system' ? 'text-blue-400' : 'text-emerald-400'}`}>
              {status.source === 'system' ? 'System PATH' : 'Orbit'}
            </span>
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
            onClick={onInstall}
            disabled={actionLoading !== null || !nodeInstalled}
            className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
          >
            {actionLoading === 'install' ? (
              <RefreshCw size={16} className="animate-spin" />
            ) : (
              <Download size={16} />
            )}
            Install
          </button>
        ) : status?.source === 'system' ? (
          <>
            <button
              onClick={onLaunch}
              disabled={actionLoading !== null}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
            >
              <ExternalLink size={16} />
              Launch
            </button>
            <button
              onClick={onInstall}
              disabled={actionLoading !== null || !nodeInstalled}
              className="flex items-center gap-2 px-4 py-2 bg-surface hover:bg-hover border border-edge rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-content-secondary"
              title="Install into Orbit for managed updates"
            >
              {actionLoading === 'install' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <Download size={16} />
              )}
              Install in Orbit
            </button>
          </>
        ) : (
          <>
            <button
              onClick={onLaunch}
              disabled={actionLoading !== null}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
            >
              <ExternalLink size={16} />
              Launch
            </button>
            <button
              onClick={onUpdate}
              disabled={actionLoading !== null}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-white"
            >
              {actionLoading === 'update' ? (
                <RefreshCw size={16} className="animate-spin" />
              ) : (
                <RefreshCw size={16} />
              )}
              Update
            </button>
            <button
              onClick={onUninstall}
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

export function AiToolManager() {
  const { addToast, setActiveTab, services, refreshAiToolStatus } = useApp()
  const [claudeStatus, setClaudeStatus] = useState<AiToolStatus | null>(null)
  const [geminiStatus, setGeminiStatus] = useState<AiToolStatus | null>(null)
  const [claudeLoading, setClaudeLoading] = useState(true)
  const [geminiLoading, setGeminiLoading] = useState(true)
  const [claudeAction, setClaudeAction] = useState<string | null>(null)
  const [geminiAction, setGeminiAction] = useState<string | null>(null)

  const nodeInstalled = services.some(s => s.service_type === 'nodejs')

  const loadClaudeStatus = async () => {
    try {
      setClaudeLoading(true)
      const result = await getClaudeCodeStatus()
      setClaudeStatus(result)
    } catch (err) {
      console.error(err)
    } finally {
      setClaudeLoading(false)
    }
  }

  const loadGeminiStatus = async () => {
    try {
      setGeminiLoading(true)
      const result = await getGeminiCliStatus()
      setGeminiStatus(result)
    } catch (err) {
      console.error(err)
    } finally {
      setGeminiLoading(false)
    }
  }

  useEffect(() => {
    loadClaudeStatus()
    loadGeminiStatus()
  }, [])

  const handleClaudeAction = async (action: 'install' | 'uninstall' | 'update') => {
    const fn = action === 'install' ? installClaudeCode : action === 'uninstall' ? uninstallClaudeCode : updateClaudeCode
    try {
      setClaudeAction(action)
      await fn()
      addToast({ type: 'success', message: `Claude Code ${action}ed successfully` })
      await loadClaudeStatus()
      refreshAiToolStatus()
    } catch (_err) {
      addToast({ type: 'error', message: `Failed to ${action} Claude Code` })
    } finally {
      setClaudeAction(null)
    }
  }

  const handleGeminiAction = async (action: 'install' | 'uninstall' | 'update') => {
    const fn = action === 'install' ? installGeminiCli : action === 'uninstall' ? uninstallGeminiCli : updateGeminiCli
    try {
      setGeminiAction(action)
      await fn()
      addToast({ type: 'success', message: `Gemini CLI ${action}ed successfully` })
      await loadGeminiStatus()
      refreshAiToolStatus()
    } catch (_err) {
      addToast({ type: 'error', message: `Failed to ${action} Gemini CLI` })
    } finally {
      setGeminiAction(null)
    }
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <AiToolCard
        name="Claude Code"
        description="AI coding assistant by Anthropic"
        icon={<Bot className="w-6 h-6 text-orange-500" />}
        iconBg="bg-orange-500/10"
        status={claudeStatus}
        loading={claudeLoading}
        actionLoading={claudeAction}
        onInstall={() => handleClaudeAction('install')}
        onUninstall={() => handleClaudeAction('uninstall')}
        onUpdate={() => handleClaudeAction('update')}
        onLaunch={() => setActiveTab('claude-code')}
        nodeInstalled={nodeInstalled}
        tooltip={
          <div className="space-y-2 text-content-secondary">
            <p>Anthropic's AI coding assistant for terminal.</p>
            <div className="space-y-1 font-mono text-xs text-content-muted">
              <p>claude</p>
              <p>claude "fix this bug"</p>
            </div>
          </div>
        }
      />
      <AiToolCard
        name="Gemini CLI"
        description="AI coding assistant by Google"
        icon={<Sparkles className="w-6 h-6 text-blue-500" />}
        iconBg="bg-blue-500/10"
        status={geminiStatus}
        loading={geminiLoading}
        actionLoading={geminiAction}
        onInstall={() => handleGeminiAction('install')}
        onUninstall={() => handleGeminiAction('uninstall')}
        onUpdate={() => handleGeminiAction('update')}
        onLaunch={() => setActiveTab('gemini-cli')}
        nodeInstalled={nodeInstalled}
        tooltip={
          <div className="space-y-2 text-content-secondary">
            <p>Google's AI coding assistant for terminal.</p>
            <div className="space-y-1 font-mono text-xs text-content-muted">
              <p>gemini</p>
              <p>gemini "explain this code"</p>
            </div>
          </div>
        }
      />
    </div>
  )
}
