import { useState, useEffect } from 'react'
import {
  deployListConnections,
  deployListTargets,
  deployAssignTarget,
  deployUnassignTarget,
  deploySync,
  deployGetStatus,
  DeployManifest,
} from '../lib/api'
import type { ServerConnection, DeployTarget } from '../lib/api'
import { listen } from '@tauri-apps/api/event'
import { useApp } from '../lib/AppContext'
import {
  Plus,
  Trash2,
  Upload,
  RefreshCw,
  Server,
  Check,
  X,
  Loader2,
} from 'lucide-react'

interface DeployPanelProps {
  domain: string
  sitePath: string
}

interface DeployProgressPayload {
  domain: string
  connection: string
  phase: string
  current: number
  total: number
  file: string | null
}

export function DeployPanel({ domain, sitePath }: DeployPanelProps) {
  const { addToast } = useApp()
  const [connections, setConnections] = useState<ServerConnection[]>([])
  const [targets, setTargets] = useState<DeployTarget[]>([])
  const [loading, setLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)
  const [deploying, setDeploying] = useState<string | null>(null)
  const [deployProgress, setDeployProgress] = useState<DeployProgressPayload | null>(null)
  const [lastDeploy, setLastDeploy] = useState<Record<string, DeployManifest | null>>({})
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  // Form state
  const [selectedConn, setSelectedConn] = useState('')
  const [remotePath, setRemotePath] = useState('/')

  const loadData = async () => {
    try {
      setLoading(true)
      const [conns, tgts] = await Promise.all([
        deployListConnections(),
        deployListTargets(domain),
      ])
      setConnections(conns)
      setTargets(tgts)

      // Load last deploy status for each target
      const statuses: Record<string, DeployManifest | null> = {}
      for (const t of tgts) {
        try {
          statuses[t.connection] = await deployGetStatus(domain, t.connection)
        } catch {
          statuses[t.connection] = null
        }
      }
      setLastDeploy(statuses)
    } catch {
      setConnections([])
      setTargets([])
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadData()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [domain])

  // Listen for deploy progress events
  useEffect(() => {
    const unlisten = listen<DeployProgressPayload>('deploy-progress', (event) => {
      if (event.payload.domain === domain) {
        setDeployProgress(event.payload)
      }
    })
    return () => { unlisten.then(fn => fn()) }
  }, [domain])

  const handleAssign = async () => {
    if (!selectedConn || !remotePath) return
    try {
      setActionLoading('assign')
      await deployAssignTarget(domain, selectedConn, remotePath)
      addToast({ type: 'success', message: `Target assigned: ${selectedConn}` })
      setSelectedConn('')
      setRemotePath('/')
      setShowForm(false)
      await loadData()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to assign target: ${err}` })
    } finally {
      setActionLoading(null)
    }
  }

  const handleUnassign = async (connName: string) => {
    try {
      setActionLoading(`remove-${connName}`)
      await deployUnassignTarget(domain, connName)
      addToast({ type: 'success', message: `Target removed: ${connName}` })
      await loadData()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to remove target: ${err}` })
    } finally {
      setActionLoading(null)
    }
  }

  const handleDeploy = async (connName: string) => {
    try {
      setDeploying(connName)
      setDeployProgress(null)
      await deploySync(domain, connName, sitePath)
      addToast({ type: 'success', message: `Deploy to "${connName}" completed` })
      await loadData()
    } catch (err) {
      addToast({ type: 'error', message: `Deploy failed: ${err}` })
    } finally {
      setDeploying(null)
      setDeployProgress(null)
    }
  }

  // Connections not yet assigned to this site
  const availableConnections = connections.filter(
    c => !targets.some(t => t.connection === c.name)
  )

  if (loading) {
    return (
      <div className="flex items-center justify-center py-4">
        <RefreshCw className="w-4 h-4 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="space-y-3">
      {/* Target List */}
      {targets.length === 0 && !showForm && (
        <div className="text-xs text-content-muted text-center py-3">
          No deploy targets configured.
          {connections.length === 0
            ? ' Add server connections in Settings first.'
            : ' Click + to assign a connection.'}
        </div>
      )}

      {targets.map(target => {
        const conn = connections.find(c => c.name === target.connection)
        const status = lastDeploy[target.connection]
        const isDeploying = deploying === target.connection

        return (
          <div
            key={target.connection}
            className="p-3 bg-surface-inset rounded-lg border border-edge/50"
          >
            <div className="flex items-center justify-between mb-1.5">
              <div className="flex items-center gap-2">
                <Server size={14} className="text-content-muted" />
                <span className="text-sm font-medium">{target.connection}</span>
                {conn && (
                  <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${conn.protocol === 'SSH' ? 'bg-emerald-500/15 text-emerald-400' : 'bg-amber-500/15 text-amber-400'}`}>
                    {conn.protocol}
                  </span>
                )}
              </div>
              <div className="flex items-center gap-1">
                <button
                  onClick={() => handleDeploy(target.connection)}
                  disabled={isDeploying || actionLoading !== null}
                  className="p-1 text-content-muted hover:text-blue-500 hover:bg-blue-500/10 rounded transition-colors disabled:opacity-50"
                  title="Deploy"
                >
                  {isDeploying ? <Loader2 size={12} className="animate-spin" /> : <Upload size={12} />}
                </button>
                <button
                  onClick={() => handleUnassign(target.connection)}
                  disabled={isDeploying || actionLoading !== null}
                  className="p-1 text-content-muted hover:text-red-500 hover:bg-red-500/10 rounded transition-colors disabled:opacity-50"
                  title="Remove target"
                >
                  {actionLoading === `remove-${target.connection}` ? <Loader2 size={12} className="animate-spin" /> : <Trash2 size={12} />}
                </button>
              </div>
            </div>
            <div className="text-xs text-content-muted space-y-0.5">
              {conn && <div>{conn.username}@{conn.host}:{conn.port}</div>}
              <div className="font-mono truncate">{target.remote_path}</div>
            </div>

            {/* Deploy Progress */}
            {isDeploying && deployProgress && (
              <div className="mt-2 p-2 bg-surface-raised rounded border border-edge/30">
                <div className="flex items-center justify-between text-[11px] mb-1">
                  <span className="text-content-secondary capitalize">{deployProgress.phase}...</span>
                  <span className="text-content-muted">
                    {deployProgress.current}/{deployProgress.total}
                  </span>
                </div>
                <div className="w-full h-1.5 bg-surface-inset rounded-full overflow-hidden">
                  <div
                    className="h-full bg-emerald-500 rounded-full transition-all duration-300"
                    style={{
                      width: deployProgress.total > 0
                        ? `${(deployProgress.current / deployProgress.total) * 100}%`
                        : '0%',
                    }}
                  />
                </div>
                {deployProgress.file && (
                  <div className="text-[10px] text-content-muted mt-1 truncate font-mono">
                    {deployProgress.file}
                  </div>
                )}
              </div>
            )}

            {/* Last Deploy Status */}
            {status && !isDeploying && (
              <div className="mt-2 flex items-center gap-1.5 text-[11px] text-content-muted">
                {status.status === 'Completed' ? (
                  <Check size={10} className="text-emerald-500" />
                ) : (
                  <X size={10} className="text-red-500" />
                )}
                <span>
                  {status.files.length} files · {new Date(status.timestamp).toLocaleDateString()}
                </span>
              </div>
            )}
          </div>
        )
      })}

      {/* Assign Target Form */}
      {showForm && (
        <div className="p-3 bg-surface-inset rounded-lg border border-edge/50 space-y-2.5">
          <div className="text-xs font-medium text-content-secondary mb-1">Assign Deploy Target</div>

          {availableConnections.length === 0 ? (
            <p className="text-xs text-content-muted">
              No available connections. Add server connections in Settings first.
            </p>
          ) : (
            <>
              <select
                value={selectedConn}
                onChange={e => setSelectedConn(e.target.value)}
                className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
              >
                <option value="">Select a connection...</option>
                {availableConnections.map(c => (
                  <option key={c.name} value={c.name}>
                    {c.name} ({c.username}@{c.host} · {c.protocol})
                  </option>
                ))}
              </select>

              <input
                type="text"
                placeholder="Remote path (e.g., /var/www/html)"
                value={remotePath}
                onChange={e => setRemotePath(e.target.value)}
                className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
              />
            </>
          )}

          <div className="flex gap-2 pt-1">
            <button
              onClick={handleAssign}
              disabled={!selectedConn || !remotePath || actionLoading === 'assign'}
              className="flex-1 flex items-center justify-center gap-1 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded text-xs font-medium text-white transition-colors disabled:opacity-50"
            >
              {actionLoading === 'assign' ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}
              Assign
            </button>
            <button
              onClick={() => { setShowForm(false); setSelectedConn(''); setRemotePath('/') }}
              className="px-3 py-1.5 bg-surface hover:bg-hover rounded text-xs text-content-muted transition-colors border border-edge"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Add button */}
      {!showForm && (
        <button
          onClick={() => setShowForm(true)}
          className="w-full flex items-center justify-center gap-1.5 px-3 py-2 bg-surface-raised hover:bg-hover border border-edge/50 rounded-lg text-xs text-content-secondary transition-colors"
        >
          <Plus size={12} />
          Add Deploy Target
        </button>
      )}
    </div>
  )
}
