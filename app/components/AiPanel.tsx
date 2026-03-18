import { useState } from 'react'
import {
  installClaudeCode,
  uninstallClaudeCode,
  installGeminiCli,
  uninstallGeminiCli,
  AiToolStatus,
} from '../lib/api'
import {
  Download,
  Trash2,
  RefreshCw,
  Bot,
  Sparkles,
  AlertTriangle,
  Info,
} from 'lucide-react'
import { useApp } from '../lib/AppContext'
import { McpManager } from './McpManager'

type ActionState = 'install' | 'uninstall' | 'update' | 'check' | null

export function AiPanel() {
  const {
    addToast,
    services,
    claudeCodeStatus,
    geminiCliStatus,
    refreshAiToolStatus,
  } = useApp()

  const [claudeAction, setClaudeAction] = useState<ActionState>(null)
  const [geminiAction, setGeminiAction] = useState<ActionState>(null)

  const nodeInstalled = services.some(s => s.service_type === 'nodejs')

  const wrap = async (
    setAction: (a: ActionState) => void,
    action: ActionState,
    fn: () => Promise<unknown>,
    successMsg: string,
    onDone?: () => void,
  ) => {
    try {
      setAction(action)
      await fn()
      addToast({ type: 'success', message: successMsg })
      onDone?.()
      await refreshAiToolStatus()
    } catch (err) {
      addToast({ type: 'error', message: `${err}` })
    } finally {
      setAction(null)
    }
  }

  const tools: {
    name: string
    icon: React.ReactNode
    color: string
    status: AiToolStatus | null
    action: ActionState
    autoUpdates?: boolean
    blocked?: string
    onInstall: () => void
    onUninstall: () => void
  }[] = [
    {
      name: 'Claude Code',
      icon: <Bot size={16} />,
      color: 'text-orange-500',
      status: claudeCodeStatus,
      action: claudeAction,
      autoUpdates: true,
      onInstall: () => wrap(setClaudeAction, 'install', installClaudeCode, 'Claude Code installed'),
      onUninstall: () => wrap(setClaudeAction, 'uninstall', uninstallClaudeCode, 'Claude Code uninstalled'),
    },
    {
      name: 'Gemini CLI',
      icon: <Sparkles size={16} />,
      color: 'text-blue-500',
      status: geminiCliStatus,
      action: geminiAction,
      autoUpdates: true,
      blocked: !nodeInstalled ? 'Node.js required' : undefined,
      onInstall: () => wrap(setGeminiAction, 'install', installGeminiCli, 'Gemini CLI installed'),
      onUninstall: () => wrap(setGeminiAction, 'uninstall', uninstallGeminiCli, 'Gemini CLI uninstalled'),
    },
  ]

  return (
    <div className="p-6 h-full flex flex-col">
      {/* Tip */}
      <div className="mb-4 p-3 bg-blue-500/5 border border-blue-500/10 rounded-lg flex items-start gap-2.5 text-xs text-blue-400">
        <Info size={14} className="shrink-0 mt-0.5" />
        <span>Open projects with AI tools directly from your <strong>Sites</strong> tab. Orbit generates context files and opens your OS terminal.</span>
      </div>

      {/* Compact tool list */}
      <div className="mb-6 bg-surface-raised border border-edge-subtle rounded-xl overflow-hidden">
        {tools.map((tool, i) => (
          <div
            key={tool.name}
            className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? 'border-t border-edge-subtle' : ''}`}
          >
            {/* Icon + Name */}
            <span className={tool.color}>{tool.icon}</span>
            <span className="text-sm font-medium w-28 shrink-0">{tool.name}</span>

            {/* Status */}
            {tool.blocked && !tool.status?.installed ? (
              <span className="flex items-center gap-1 text-[11px] text-amber-400">
                <AlertTriangle size={11} />
                {tool.blocked}
              </span>
            ) : !tool.status?.installed ? (
              <span className="text-[11px] text-content-muted">Not installed</span>
            ) : (
              <div className="flex items-center gap-2 text-[11px]">
                <span className={`px-1.5 py-0.5 rounded ${tool.status.source === 'system' ? 'bg-blue-500/15 text-blue-400' : tool.status.source === 'native' ? 'bg-emerald-500/15 text-emerald-400' : 'bg-emerald-500/15 text-emerald-400'}`}>
                  {tool.status.source === 'system' ? 'System' : tool.status.source === 'native' ? 'Native' : 'Orbit'}
                </span>
                <span className="font-mono text-content-secondary">{tool.status.version || '?'}</span>
                {tool.autoUpdates && (
                  <span className="text-content-muted">auto-updates</span>
                )}
              </div>
            )}

            {/* Spacer */}
            <div className="flex-1" />

            {/* Actions — icon buttons only */}
            <div className="flex items-center gap-1">
              {!tool.status?.installed ? (
                <IconBtn
                  title="Install"
                  disabled={tool.action !== null || !!tool.blocked}
                  spinning={tool.action === 'install'}
                  onClick={tool.onInstall}
                  className="text-emerald-500 hover:bg-emerald-500/10"
                >
                  <Download size={14} />
                </IconBtn>
              ) : (
                <>
                  <IconBtn
                    title="Uninstall"
                    disabled={tool.action !== null}
                    spinning={tool.action === 'uninstall'}
                    onClick={tool.onUninstall}
                    className="text-red-500/60 hover:bg-red-500/10 hover:text-red-500"
                  >
                    <Trash2 size={14} />
                  </IconBtn>
                </>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        <McpManager />
      </div>
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
