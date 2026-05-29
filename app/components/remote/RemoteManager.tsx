import { useState, useEffect, useCallback } from 'react'
import {
  Network,
  TerminalSquare,
  Plug,
  PlugZap,
  ChevronDown,
  Loader2,
} from 'lucide-react'
import clsx from 'clsx'
import { deployListConnections, sftpDisconnect, type ServerConnection } from '../../lib/api'
import { useApp } from '../../lib/AppContext'
import { DualPaneExplorer } from './DualPaneExplorer'
import { SshTerminalPane } from './SshTerminalPane'

const TERM_HEIGHT = 280

export function RemoteManager() {
  const { addToast } = useApp()
  const [connections, setConnections] = useState<ServerConnection[]>([])
  const [loading, setLoading] = useState(true)
  const [selected, setSelected] = useState<string | null>(null)
  const [pickerOpen, setPickerOpen] = useState(false)
  const [termOpen, setTermOpen] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const all = await deployListConnections()
      const ssh = all.filter((c) => c.protocol === 'SSH')
      setConnections(ssh)
    } catch (e) {
      addToast({ type: 'error', message: `Failed to load connections: ${e}` })
    } finally {
      setLoading(false)
    }
  }, [addToast])

  useEffect(() => {
    load()
  }, [load])

  const connect = (name: string) => {
    setSelected(name)
    setPickerOpen(false)
  }

  const disconnect = async () => {
    if (!selected) return
    setTermOpen(false)
    try {
      await sftpDisconnect(selected)
    } catch {
      /* best effort */
    }
    setSelected(null)
  }

  const selectedConn = connections.find((c) => c.name === selected)

  return (
    <div className="flex flex-col h-full">
      {/* Connection bar */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-edge bg-surface shrink-0">
        <div className="flex items-center gap-2">
          <Network className="w-4 h-4 text-emerald-500" />
          <span className="text-sm font-semibold text-content-secondary">Remote</span>
        </div>

        <div className="flex items-center gap-2">
          {/* Connection picker */}
          <div className="relative">
            <button
              onClick={() => setPickerOpen((o) => !o)}
              className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-surface-raised border border-edge rounded-md hover:bg-hover transition-colors"
            >
              <span
                className={clsx(
                  'w-1.5 h-1.5 rounded-full',
                  selected ? 'bg-emerald-500' : 'bg-zinc-500',
                )}
              />
              <span className="text-content-secondary">
                {selectedConn
                  ? `${selectedConn.username}@${selectedConn.host}`
                  : 'Select connection'}
              </span>
              <ChevronDown className="w-3.5 h-3.5 text-content-muted" />
            </button>

            {pickerOpen && (
              <>
                <div className="fixed inset-0 z-40" onClick={() => setPickerOpen(false)} />
                <div className="absolute right-0 top-full mt-1 w-64 max-h-72 overflow-y-auto custom-scrollbar bg-surface border border-edge rounded-md shadow-xl z-50 py-1">
                  {loading && (
                    <div className="flex items-center justify-center py-4 text-content-muted">
                      <Loader2 className="w-4 h-4 animate-spin" />
                    </div>
                  )}
                  {!loading && connections.length === 0 && (
                    <div className="px-3 py-3 text-xs text-content-muted text-center">
                      No SSH connections.
                      <br />
                      Add one in a site's Deploy panel.
                    </div>
                  )}
                  {connections.map((c) => (
                    <button
                      key={c.name}
                      onClick={() => connect(c.name)}
                      className={clsx(
                        'w-full flex flex-col items-start px-3 py-2 hover:bg-hover transition-colors text-left',
                        selected === c.name && 'bg-emerald-500/10',
                      )}
                    >
                      <span className="text-xs font-medium text-content-secondary">{c.name}</span>
                      <span className="text-[11px] text-content-muted font-mono">
                        {c.username}@{c.host}:{c.port}
                      </span>
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>

          {selected && (
            <>
              <button
                onClick={() => setTermOpen((o) => !o)}
                className={clsx(
                  'flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium rounded-md transition-colors border',
                  termOpen
                    ? 'text-emerald-500 border-emerald-500/40 bg-emerald-500/10'
                    : 'text-content-muted border-edge hover:bg-hover',
                )}
                title="Toggle SSH terminal"
              >
                <TerminalSquare className="w-3.5 h-3.5" />
                Terminal
              </button>
              <button
                onClick={disconnect}
                className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium text-red-400 border border-edge rounded-md hover:bg-red-500/10 transition-colors"
                title="Disconnect"
              >
                <PlugZap className="w-3.5 h-3.5" />
                Disconnect
              </button>
            </>
          )}
        </div>
      </div>

      {/* Body */}
      {selectedConn ? (
        <div className="flex flex-col flex-1 min-h-0">
          <DualPaneExplorer connection={selectedConn.name} />
          {termOpen && (
            <div
              className="border-t border-edge shrink-0 flex flex-col"
              style={{ height: TERM_HEIGHT }}
            >
              <div className="flex items-center gap-2 px-3 py-1 bg-surface-raised border-b border-edge text-[11px] text-content-muted">
                <TerminalSquare className="w-3 h-3 text-emerald-500" />
                SSH · {selectedConn.username}@{selectedConn.host}
              </div>
              <div className="flex-1 min-h-0">
                <SshTerminalPane connection={selectedConn.name} />
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="flex flex-1 flex-col items-center justify-center text-content-muted gap-3">
          <Plug className="w-10 h-10 opacity-30" />
          <p className="text-sm">Select an SSH connection to browse files and open a shell.</p>
        </div>
      )}
    </div>
  )
}
