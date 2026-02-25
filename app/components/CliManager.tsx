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
    ArrowUpCircle
} from 'lucide-react'
import { useApp } from '../lib/AppContext'
import { InfoTooltip } from './InfoTooltip'

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

    useEffect(() => {
        loadStatus()
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    const handleInstall = async () => {
        try {
            setActionLoading('install')
            await installCli()
            addToast({ type: 'success', message: 'CLI installed successfully. Restart terminal to use orbit-cli.' })
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
            addToast({ type: 'success', message: 'CLI updated successfully. Restart terminal to use new version.' })
            setUpdateInfo(null)
            await loadStatus()
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
                            <p>Orbit CLI lets you manage services, sites, databases, and more directly from your terminal.</p>
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
                            Installed{status.version ? ` • ${status.version}` : ''}
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
                    <div className="flex justify-between">
                        <span className="text-content-muted">Path:</span>
                        <span className="font-mono text-xs truncate max-w-[300px]">{status.path}</span>
                    </div>
                </div>
            )}

            {/* Usage examples when installed */}
            {status?.installed && (
                <div className="mb-4">
                    <p className="text-sm font-medium mb-2">Quick Commands</p>
                    <div className="p-3 bg-surface-inset rounded-lg text-xs font-mono space-y-1 border border-edge-subtle">
                        <div><span className="text-content-muted">$</span> orbit-cli status</div>
                        <div><span className="text-content-muted">$</span> orbit-cli start nginx</div>
                        <div><span className="text-content-muted">$</span> orbit-cli sites</div>
                        <div><span className="text-content-muted">$</span> orbit-cli db list</div>
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
