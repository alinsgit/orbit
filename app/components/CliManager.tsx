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
    Check
} from 'lucide-react'
import { useApp } from '../lib/AppContext'
import { InfoTooltip } from './InfoTooltip'

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
            { cmd: 'orbit-cli stop --all', desc: 'Stop all running services' },
            { cmd: 'orbit-cli restart <service>', desc: 'Restart a service' },
            { cmd: 'orbit-cli restart --all', desc: 'Restart all services' },
            { cmd: 'orbit-cli list', desc: 'List available services to install' },
            { cmd: 'orbit-cli install <service>', desc: 'Install a service (nginx, php, redis…)' },
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
        label: 'Logs',
        commands: [
            { cmd: 'orbit-cli logs list', desc: 'List all log files with sizes' },
            { cmd: 'orbit-cli logs show <name>', desc: 'Show log contents (e.g. nginx/error.log)' },
            { cmd: 'orbit-cli logs clear <name>', desc: 'Clear a log file' },
        ],
    },
    {
        label: 'PHP & Composer',
        commands: [
            { cmd: 'orbit-cli php list', desc: 'List installed PHP versions' },
            { cmd: 'orbit-cli php ext <version>', desc: 'Manage PHP extensions for a version' },
            { cmd: 'orbit-cli composer <args>', desc: "Run Composer using Orbit's PHP" },
        ],
    },
    {
        label: 'Sites & Hosts',
        commands: [
            { cmd: 'orbit-cli sites', desc: 'List all configured local sites' },
            { cmd: 'orbit-cli sites --json', desc: 'Output sites as JSON' },
            { cmd: 'orbit-cli hosts list', desc: 'Show hosts file entries' },
            { cmd: 'orbit-cli hosts add <domain>', desc: 'Add a domain to hosts file' },
            { cmd: 'orbit-cli hosts remove <domain>', desc: 'Remove a domain from hosts file' },
            { cmd: 'orbit-cli open <target>', desc: 'Open a site or tool in browser' },
        ],
    },
    {
        label: 'System',
        commands: [
            { cmd: 'orbit-cli info', desc: 'Show environment info and paths' },
        ],
    },
]

// Aliases note
const ALIASES = 'Aliases: pg / postgres → postgresql · maria / mysql → mariadb · mongo → mongodb · node → nodejs'

// ─── CopyableCommand ──────────────────────────────────────────────

function CopyableCommand({ cmd, desc }: CliCommand) {
    const [copied, setCopied] = useState(false)
    const { addToast } = useApp()

    const handleCopy = async () => {
        try {
            await navigator.clipboard.writeText(cmd)
            setCopied(true)
            setTimeout(() => setCopied(false), 1500)
        } catch {
            addToast({ type: 'error', message: 'Failed to copy' })
        }
    }

    return (
        <button
            onClick={handleCopy}
            className="w-full text-left group flex items-start gap-3 px-3 py-2 rounded-lg hover:bg-surface transition-colors"
            title="Click to copy"
        >
            <div className="flex-1 min-w-0">
                <code className="text-xs font-mono text-emerald-400 break-all">{cmd}</code>
                <p className="text-xs text-content-muted mt-0.5 leading-snug">{desc}</p>
            </div>
            <div className="shrink-0 mt-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                {copied
                    ? <Check size={13} className="text-emerald-400" />
                    : <Copy size={13} className="text-content-muted" />}
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
    const [openCategory, setOpenCategory] = useState<string | null>('Services')

    const loadStatus = async () => {
        try {
            setLoading(true)
            const result = await getCliStatus()
            setStatus(result)
            if (result.installed) {
                checkCliUpdate().then(setUpdateInfo).catch(() => { })
            }
        } catch (err) {
            addToast({ type: 'error', message: 'Failed to load CLI status' })
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
            await loadStatus()
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
            setTimeout(() => loadStatus(), 800)
        } catch (_err) {
            addToast({ type: 'error', message: 'Failed to update CLI' })
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
            {/* Header */}
            <div className="flex items-center gap-3 mb-4">
                <div className="p-3 bg-sky-500/10 rounded-xl">
                    <TerminalSquare className="w-6 h-6 text-sky-500" />
                </div>
                <div>
                    <h2 className="text-lg font-semibold">Orbit CLI</h2>
                    <p className="text-content-secondary text-sm">Terminal command-line tool</p>
                </div>
                <InfoTooltip
                    content={
                        <div className="space-y-2 text-content-secondary">
                            <p>Orbit CLI lets you manage services, sites, databases, and more directly from your terminal — even when the GUI is closed.</p>
                            <p className="text-xs text-content-muted">After installing, open a new terminal and run <code className="bg-surface-inset px-1 rounded">orbit-cli status</code></p>
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
                        <span className="px-2 py-1 bg-emerald-500/20 text-emerald-400 rounded text-xs font-medium">
                            Installed{status.version ? ` · ${status.version}` : ''}
                        </span>
                    ) : (
                        <span className="px-2 py-1 bg-gray-500/20 text-gray-400 rounded text-xs font-medium">
                            Not Installed
                        </span>
                    )}
                </div>
            </div>

            {/* Path row */}
            {status?.installed && (
                <div className="mb-4 p-3 bg-surface-inset rounded-lg text-sm">
                    <div className="flex justify-between">
                        <span className="text-content-muted">Path:</span>
                        <span className="font-mono text-xs truncate max-w-[300px]">{status.path}</span>
                    </div>
                </div>
            )}

            {/* Command Reference */}
            {status?.installed && (
                <div className="mb-4">
                    <p className="text-sm font-medium mb-2">
                        Command Reference
                        <span className="ml-2 text-xs font-normal text-content-muted">(click any command to copy)</span>
                    </p>
                    <div className="border border-edge-subtle rounded-lg overflow-hidden divide-y divide-edge-subtle">
                        {CLI_REFERENCE.map((category) => {
                            const isOpen = openCategory === category.label
                            return (
                                <div key={category.label}>
                                    {/* Category header */}
                                    <button
                                        onClick={() => setOpenCategory(isOpen ? null : category.label)}
                                        className="w-full flex items-center justify-between px-3 py-2 bg-surface-inset hover:bg-hover transition-colors text-left"
                                    >
                                        <span className="text-xs font-semibold text-content-secondary uppercase tracking-wider">
                                            {category.label}
                                        </span>
                                        <span className="text-content-muted text-xs">
                                            {isOpen ? '▲' : '▼'}
                                        </span>
                                    </button>

                                    {/* Commands */}
                                    {isOpen && (
                                        <div className="bg-surface-raised divide-y divide-edge-subtle/50">
                                            {category.commands.map((c) => (
                                                <CopyableCommand key={c.cmd} {...c} />
                                            ))}
                                        </div>
                                    )}
                                </div>
                            )
                        })}
                    </div>

                    {/* Aliases footnote */}
                    <p className="text-xs text-content-muted mt-2 px-1">{ALIASES}</p>
                </div>
            )}

            {/* Not installed: show preview */}
            {!status?.installed && (
                <div className="mb-4 p-3 bg-surface-inset rounded-lg border border-edge-subtle">
                    <p className="text-xs text-content-muted mb-2">After installing, you'll have access to commands like:</p>
                    <div className="space-y-1 font-mono text-xs text-emerald-400">
                        <div>$ orbit-cli status</div>
                        <div>$ orbit-cli start nginx</div>
                        <div>$ orbit-cli db list</div>
                        <div>$ orbit-cli logs show nginx/error.log</div>
                        <div className="text-content-muted">…and 20+ more commands</div>
                    </div>
                </div>
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
                            className="flex items-center gap-2 px-4 py-2 bg-red-600/10 hover:bg-red-600/20 text-red-500 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
                        >
                            {actionLoading === 'uninstall' ? (
                                <RefreshCw size={16} className="animate-spin" />
                            ) : (
                                <Trash2 size={16} />
                            )}
                            Uninstall
                        </button>
                    </>
                )}
            </div>
        </div>
    )
}
