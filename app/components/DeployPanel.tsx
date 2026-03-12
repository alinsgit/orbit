import { useState, useEffect } from 'react'
import {
  deployListConnections,
  deployAddConnection,
  deployRemoveConnection,
  deployTestConnection,
  deploySync,
  deployGetStatus,
  DeployConnection,
  DeployManifest,
} from '../lib/api'
import { listen } from '@tauri-apps/api/event'
import { useApp } from '../lib/AppContext'
import {
  Plus,
  Trash2,
  Wifi,
  Upload,
  RefreshCw,
  Server,
  Key,
  FileKey,
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

const DEFAULT_PORTS: Record<string, number> = {
  SSH: 22,
  SFTP: 22,
  FTP: 21,
}

export function DeployPanel({ domain, sitePath }: DeployPanelProps) {
  const { addToast } = useApp()
  const [connections, setConnections] = useState<DeployConnection[]>([])
  const [loading, setLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)
  const [testing, setTesting] = useState<string | null>(null)
  const [deploying, setDeploying] = useState<string | null>(null)
  const [deployProgress, setDeployProgress] = useState<DeployProgressPayload | null>(null)
  const [lastDeploy, setLastDeploy] = useState<Record<string, DeployManifest | null>>({})

  // Form state
  const [form, setForm] = useState({
    name: '',
    protocol: 'SSH' as 'SSH' | 'SFTP' | 'FTP',
    host: '',
    port: 22,
    username: '',
    authType: 'password' as 'password' | 'keyfile',
    password: '',
    keyFilePath: '',
    remotePath: '/',
  })
  const [formSaving, setFormSaving] = useState(false)

  const loadConnections = async () => {
    try {
      setLoading(true)
      const conns = await deployListConnections(domain)
      setConnections(conns)
      // Load last deploy status for each
      const statuses: Record<string, DeployManifest | null> = {}
      for (const conn of conns) {
        try {
          statuses[conn.name] = await deployGetStatus(domain, conn.name)
        } catch {
          statuses[conn.name] = null
        }
      }
      setLastDeploy(statuses)
    } catch {
      // No connections yet
      setConnections([])
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadConnections()
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

  const resetForm = () => {
    setForm({
      name: '',
      protocol: 'SSH',
      host: '',
      port: 22,
      username: '',
      authType: 'password',
      password: '',
      keyFilePath: '',
      remotePath: '/',
    })
  }

  const handleAdd = async () => {
    if (!form.name || !form.host || !form.username) {
      addToast({ type: 'warning', message: 'Name, host, and username are required' })
      return
    }

    try {
      setFormSaving(true)
      const connection: DeployConnection = {
        name: form.name,
        protocol: form.protocol,
        host: form.host,
        port: form.port,
        username: form.username,
        auth: form.authType === 'keyfile' ? { KeyFile: form.keyFilePath } : 'Password',
        remote_path: form.remotePath,
      }
      await deployAddConnection(
        domain,
        connection,
        form.authType === 'password' ? form.password : undefined,
      )
      addToast({ type: 'success', message: `Connection "${form.name}" added` })
      resetForm()
      setShowForm(false)
      await loadConnections()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to add connection: ${err}` })
    } finally {
      setFormSaving(false)
    }
  }

  const handleRemove = async (connName: string) => {
    try {
      await deployRemoveConnection(domain, connName)
      addToast({ type: 'success', message: `Connection "${connName}" removed` })
      await loadConnections()
    } catch (err) {
      addToast({ type: 'error', message: `Failed to remove: ${err}` })
    }
  }

  const handleTest = async (connName: string) => {
    try {
      setTesting(connName)
      const result = await deployTestConnection(domain, connName)
      addToast({ type: 'success', message: result })
    } catch (err) {
      addToast({ type: 'error', message: `Connection test failed: ${err}` })
    } finally {
      setTesting(null)
    }
  }

  const handleDeploy = async (connName: string) => {
    try {
      setDeploying(connName)
      setDeployProgress(null)
      await deploySync(domain, connName, sitePath)
      addToast({ type: 'success', message: `Deploy to "${connName}" completed` })
      await loadConnections()
    } catch (err) {
      addToast({ type: 'error', message: `Deploy failed: ${err}` })
    } finally {
      setDeploying(null)
      setDeployProgress(null)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-4">
        <RefreshCw className="w-4 h-4 animate-spin text-emerald-500" />
      </div>
    )
  }

  return (
    <div className="space-y-3">
      {/* Connection List */}
      {connections.length === 0 && !showForm && (
        <div className="text-xs text-content-muted text-center py-3">
          No deploy connections configured
        </div>
      )}

      {connections.map(conn => {
        const status = lastDeploy[conn.name]
        const isDeploying = deploying === conn.name
        const isTesting = testing === conn.name

        return (
          <div
            key={conn.name}
            className="p-3 bg-surface-inset rounded-lg border border-edge/50"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <Server size={14} className="text-content-muted" />
                <span className="text-sm font-medium">{conn.name}</span>
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-surface-raised text-content-muted">
                  {conn.protocol}
                </span>
              </div>
              <div className="flex items-center gap-1">
                <button
                  onClick={() => handleTest(conn.name)}
                  disabled={isTesting || isDeploying}
                  className="p-1 text-content-muted hover:text-emerald-500 hover:bg-emerald-500/10 rounded transition-colors disabled:opacity-50"
                  title="Test Connection"
                >
                  {isTesting ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Wifi size={12} />
                  )}
                </button>
                <button
                  onClick={() => handleDeploy(conn.name)}
                  disabled={isTesting || isDeploying}
                  className="p-1 text-content-muted hover:text-blue-500 hover:bg-blue-500/10 rounded transition-colors disabled:opacity-50"
                  title="Deploy"
                >
                  {isDeploying ? (
                    <Loader2 size={12} className="animate-spin" />
                  ) : (
                    <Upload size={12} />
                  )}
                </button>
                <button
                  onClick={() => handleRemove(conn.name)}
                  disabled={isTesting || isDeploying}
                  className="p-1 text-content-muted hover:text-red-500 hover:bg-red-500/10 rounded transition-colors disabled:opacity-50"
                  title="Remove"
                >
                  <Trash2 size={12} />
                </button>
              </div>
            </div>
            <div className="text-xs text-content-muted space-y-0.5">
              <div>{conn.username}@{conn.host}:{conn.port}</div>
              <div className="font-mono truncate">{conn.remote_path}</div>
              <div className="flex items-center gap-1">
                {conn.auth === 'Password' ? (
                  <><Key size={10} /> Password</>
                ) : (
                  <><FileKey size={10} /> Key File</>
                )}
              </div>
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

      {/* Add Connection Form */}
      {showForm && (
        <div className="p-3 bg-surface-inset rounded-lg border border-edge/50 space-y-2.5">
          <div className="text-xs font-medium text-content-secondary mb-2">New Connection</div>

          <input
            type="text"
            placeholder="Connection name"
            value={form.name}
            onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
            className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
          />

          <div className="grid grid-cols-2 gap-2">
            <select
              value={form.protocol}
              onChange={e => {
                const protocol = e.target.value as 'SSH' | 'SFTP' | 'FTP'
                setForm(f => ({ ...f, protocol, port: DEFAULT_PORTS[protocol] }))
              }}
              className="px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
            >
              <option value="SSH">SSH</option>
              <option value="SFTP">SFTP</option>
              <option value="FTP">FTP</option>
            </select>
            <input
              type="number"
              placeholder="Port"
              value={form.port}
              onChange={e => setForm(f => ({ ...f, port: parseInt(e.target.value) || 22 }))}
              className="px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
            />
          </div>

          <input
            type="text"
            placeholder="Host (e.g., example.com)"
            value={form.host}
            onChange={e => setForm(f => ({ ...f, host: e.target.value }))}
            className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
          />

          <input
            type="text"
            placeholder="Username"
            value={form.username}
            onChange={e => setForm(f => ({ ...f, username: e.target.value }))}
            className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
          />

          <div className="flex gap-2">
            <button
              onClick={() => setForm(f => ({ ...f, authType: 'password' }))}
              className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded text-xs transition-colors ${
                form.authType === 'password'
                  ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                  : 'bg-surface border border-edge text-content-muted hover:text-content-secondary'
              }`}
            >
              <Key size={10} /> Password
            </button>
            <button
              onClick={() => setForm(f => ({ ...f, authType: 'keyfile' }))}
              className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded text-xs transition-colors ${
                form.authType === 'keyfile'
                  ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                  : 'bg-surface border border-edge text-content-muted hover:text-content-secondary'
              }`}
            >
              <FileKey size={10} /> Key File
            </button>
          </div>

          {form.authType === 'password' ? (
            <input
              type="password"
              placeholder="Password"
              value={form.password}
              onChange={e => setForm(f => ({ ...f, password: e.target.value }))}
              className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
            />
          ) : (
            <input
              type="text"
              placeholder="Key file path (e.g., ~/.ssh/id_rsa)"
              value={form.keyFilePath}
              onChange={e => setForm(f => ({ ...f, keyFilePath: e.target.value }))}
              className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
            />
          )}

          <input
            type="text"
            placeholder="Remote path (e.g., /var/www/html)"
            value={form.remotePath}
            onChange={e => setForm(f => ({ ...f, remotePath: e.target.value }))}
            className="w-full px-2.5 py-1.5 bg-surface rounded border border-edge text-xs focus:border-emerald-500 focus:outline-none"
          />

          <div className="flex gap-2 pt-1">
            <button
              onClick={handleAdd}
              disabled={formSaving}
              className="flex-1 flex items-center justify-center gap-1 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 rounded text-xs font-medium text-white transition-colors disabled:opacity-50"
            >
              {formSaving ? <Loader2 size={12} className="animate-spin" /> : <Check size={12} />}
              Save
            </button>
            <button
              onClick={() => { setShowForm(false); resetForm() }}
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
          Add Connection
        </button>
      )}
    </div>
  )
}
