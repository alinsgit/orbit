import { useState, useEffect } from 'react'
import {
  getCliStatus,
  installCli,
  uninstallCli,
  checkCliUpdate,
  updateCli,
  CliStatus,
  BinaryUpdateInfo
} from '../lib/api'
import {
  Download,
  Trash2,
  RefreshCw,
  TerminalSquare,
  ArrowUpCircle,
  Copy,
  Check,
  ChevronDown,
  ChevronUp
} from 'lucide-react'
import { useApp } from '../lib/AppContext'

// ─── Command Reference ─────────────────────────────────────────────

interface CliCommand {
  cmd: string
  desc: string
}

interface CliCategory {
  label: string
  commands: CliCommand[]
}

const CLI_REFERENCE: CliCategory[] = [
  {
    label: 'Services',
    commands: [
      { cmd: 'orbit-cli status', desc: 'Show all services and their status' },
      { cmd: 'orbit-cli start <service>', desc: 'Start a service (e.g. nginx, mariadb)' },
      { cmd: 'orbit-cli start --all', desc: 'Start all installed services' },
      { cmd: 'orbit-cli stop <service>', desc: 'Stop a service' },
      { cmd: 'orbit-cli restart <service>', desc: 'Restart a service' },
      { cmd: 'orbit-cli list', desc: 'List available services to install' },
      { cmd: 'orbit-cli install <service>', desc: 'Install a service' },
      { cmd: 'orbit-cli uninstall <service>', desc: 'Uninstall a service' },
    ],
  },
  {
    label: 'Database',
    commands: [
      { cmd: 'orbit-cli db list', desc: 'List all MariaDB databases' },
      { cmd: 'orbit-cli db create <name>', desc: 'Create a new database' },
      { cmd: 'orbit-cli db drop <name>', desc: 'Drop a database' },
      { cmd: 'orbit-cli db export <name>', desc: 'Export database to SQL file' },
      { cmd: 'orbit-cli db import <name> <file>', desc: 'Import SQL file into database' },
    ],
  },
  {
    label: 'Logs & PHP',
    commands: [
      { cmd: 'orbit-cli logs list', desc: 'List all log files with sizes' },
      { cmd: 'orbit-cli logs show <name>', desc: 'Show log contents' },
      { cmd: 'orbit-cli php list', desc: 'List installed PHP versions' },
      { cmd: 'orbit-cli composer <args>', desc: "Run Composer using Orbit's PHP" },
    ],
  },
  {
    label: 'Sites & System',
    commands: [
      { cmd: 'orbit-cli sites', desc: 'List all configured local sites' },
      { cmd: 'orbit-cli hosts add <domain>', desc: 'Add a domain to hosts file' },
      { cmd: 'orbit-cli open <target>', desc: 'Open a site or tool in browser' },
      { cmd: 'orbit-cli info', desc: 'Show environment info and paths' },
    ],
  },
]

const ALIASES = 'Aliases: pg → postgresql · maria → mariadb · mongo → mongodb · node → nodejs'

// ─── CopyableCommand ──────────────────────────────────────────────

function CopyableCommand({ cmd, desc }: CliCommand) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(cmd)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch { /* noop */ }
  }

  return (
    <button
      onClick={handleCopy}
      className="w-full text-left group flex items-center gap-2 px-2.5 py-1.5 hover:bg-hover transition-colors"
      title="Click to copy"
    >
      <code className="text-[11px] font-mono text-emerald-400 flex-1 truncate">{cmd}</code>
      <span className="text-[10px] text-content-muted hidden sm:block truncate max-w-[140px]">{desc}</span>
      <div className="shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
        {copied
          ? <Check size={11} className="text-emerald-400" />
          : <Copy size={11} className="text-content-muted" />}
      </div>
    </button>
  )
}

// ─── CliManager ───────────────────────────────────────────────────

export function CliManager() {
  const { addToast } = useApp()
  const [status, setStatus] = useState<CliStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string | null>(null)
  const [updateInfo, setUpdateInfo] = useState<BinaryUpdateInfo | null>(null)

  const loadStatus = async () => {
    try {
      setLoading(true)
      const result = await getCliStatus()
      setStatus(result)
      if (result.installed) {
        checkCliUpdate().then(setUpdateInfo).catch(() => {})
      }
    } catch (err) {
      addToast({ type: 'error', message: 'Failed to load CLI status' })
      console.error(err)
    } finally {
      setLoading(false)
    }
  }

  const reloadStatusOnly = async () => {
    try {
      const result = await getCliStatus()
      setStatus(result)
    } catch { /* noop */ }
  }

  useEffect(() => {
    loadStatus()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleInstall = async () => {
    try {
      setActionLoading('install')
      await installCli()
      addToast({ type: 'success', message: 'CLI installed. Restart terminal to use orbit-cli.' })
      await loadStatus()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to install CLI' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUninstall = async () => {
    try {
      setActionLoading('uninstall')
      await uninstallCli()
      addToast({ type: 'success', message: 'CLI uninstalled' })
      setUpdateInfo(null)
      await reloadStatusOnly()
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to uninstall CLI' })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUpdate = async () => {
    try {
      setActionLoading('update')
      await updateCli()
      addToast({ type: 'success', message: 'CLI updated. Restart terminal to use new version.' })
      setUpdateInfo(null)
      setTimeout(() => reloadStatusOnly(), 800)
    } catch (_err) {
      addToast({ type: 'error', message: 'Failed to update CLI' })
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
        <div className="p-2 bg-sky-500/10 rounded-lg">
          <TerminalSquare className="w-4 h-4 text-sky-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold">Orbit CLI</span>
            {status?.installed ? (
              <span className="px-1.5 py-0.5 bg-emerald-500/15 text-emerald-400 rounded text-[10px] font-medium">
                {status.version || 'Installed'}
              </span>
            ) : (
              <span className="px-1.5 py-0.5 bg-zinc-500/15 text-zinc-400 rounded text-[10px] font-medium">
                Not Installed
              </span>
            )}
            {updateInfo?.has_update && (
              <span className="px-1.5 py-0.5 bg-amber-500/15 text-amber-400 rounded text-[10px] font-medium">
                v{updateInfo.latest_version}
              </span>
            )}
          </div>
          <p className="text-[11px] text-content-muted leading-tight">Terminal command-line tool</p>
        </div>
      </div>

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
            {updateInfo?.has_update && (
              <button
                onClick={handleUpdate}
                disabled={actionLoading !== null}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-amber-600/15 hover:bg-amber-600/25 text-amber-400 rounded-lg text-xs font-medium transition-colors disabled:opacity-50"
              >
                {actionLoading === 'update' ? <RefreshCw size={13} className="animate-spin" /> : <ArrowUpCircle size={13} />}
                Update
              </button>
            )}
            <button
              onClick={handleUninstall}
              disabled={actionLoading !== null}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg text-xs font-medium transition-colors disabled:opacity-50"
            >
              {actionLoading === 'uninstall' ? <RefreshCw size={13} className="animate-spin" /> : <Trash2 size={13} />}
              Uninstall
            </button>
          </>
        )}
      </div>
    </div>
  )
}

// ─── CliCommandReference (exported separately for Tools tab) ──────

export function CliCommandReference() {
  const [openCategory, setOpenCategory] = useState<string | null>(null)
  const [cliInstalled, setCliInstalled] = useState(false)

  useEffect(() => {
    getCliStatus().then(s => setCliInstalled(s.installed)).catch(() => {})
  }, [])

  if (!cliInstalled) return null

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4">
      <div className="flex items-center gap-2 mb-3">
        <TerminalSquare className="w-4 h-4 text-sky-500" />
        <span className="text-sm font-semibold">CLI Command Reference</span>
        <span className="text-[10px] text-content-muted">(click to copy)</span>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
        {CLI_REFERENCE.map((category) => {
          const isOpen = openCategory === category.label
          return (
            <div key={category.label} className="border border-edge-subtle rounded-lg overflow-hidden">
              <button
                onClick={() => setOpenCategory(isOpen ? null : category.label)}
                className="w-full flex items-center justify-between px-3 py-1.5 bg-surface-inset hover:bg-hover transition-colors text-left"
              >
                <span className="text-[11px] font-semibold text-content-secondary uppercase tracking-wider">
                  {category.label}
                </span>
                {isOpen ? <ChevronUp size={12} className="text-content-muted" /> : <ChevronDown size={12} className="text-content-muted" />}
              </button>
              {isOpen && (
                <div className="divide-y divide-edge-subtle/50">
                  {category.commands.map((c) => (
                    <CopyableCommand key={c.cmd} {...c} />
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>

      <p className="text-[10px] text-content-muted mt-2">{ALIASES}</p>
    </div>
  )
}
