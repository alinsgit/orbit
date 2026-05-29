import { useState, useEffect, useCallback } from 'react'
import { Monitor, Server, Upload, Download } from 'lucide-react'
import {
  localListDir,
  localMkdir,
  localDelete,
  localRename,
  localHomeDir,
  sftpListDir,
  sftpMkdir,
  sftpDelete,
  sftpRename,
  sftpUploadPath,
  sftpDownloadPath,
  type FileEntry,
  type DirListing,
} from '../../lib/api'
import { useApp } from '../../lib/AppContext'
import { FilePane } from './FilePane'
import { PromptModal, type PromptState } from './PromptModal'

interface DualPaneExplorerProps {
  connection: string
}

type Sep = '/' | '\\'

/** Last path segment (file/dir name). */
function baseName(path: string): string {
  const trimmed = path.replace(/[/\\]+$/, '')
  const idx = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  return idx >= 0 ? trimmed.slice(idx + 1) : trimmed
}

/** Parent directory, preserving the platform separator. */
function parentPath(path: string, sep: Sep): string {
  const trimmed = path.replace(/[/\\]+$/, '')
  const idx = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  if (idx <= 0) return sep === '/' ? '/' : trimmed.slice(0, idx + 1) || trimmed
  return trimmed.slice(0, idx)
}

function joinPath(dir: string, name: string, sep: Sep): string {
  return `${dir.replace(/[/\\]+$/, '')}${sep}${name}`
}

/** Per-pane directory state + operations. */
function useDirPane(load: (path: string) => Promise<DirListing>) {
  const [path, setPath] = useState('')
  const [entries, setEntries] = useState<FileEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [selected, setSelected] = useState<Set<string>>(new Set())

  const navigate = useCallback(
    async (target: string) => {
      setLoading(true)
      setError(null)
      try {
        const listing = await load(target)
        setEntries(listing.entries)
        setPath(listing.path)
        setSelected(new Set())
      } catch (e) {
        setError(String(e))
      } finally {
        setLoading(false)
      }
    },
    [load],
  )

  const toggleSelect = useCallback(
    (entry: FileEntry, e: React.MouseEvent) => {
      setSelected((prev) => {
        const next = new Set(prev)
        if (e.ctrlKey || e.metaKey) {
          if (next.has(entry.path)) next.delete(entry.path)
          else next.add(entry.path)
        } else {
          next.clear()
          next.add(entry.path)
        }
        return next
      })
    },
    [],
  )

  return {
    path,
    entries,
    loading,
    error,
    selected,
    navigate,
    toggleSelect,
    setSelected,
  }
}

export function DualPaneExplorer({ connection }: DualPaneExplorerProps) {
  const { addToast } = useApp()
  const [busy, setBusy] = useState<'upload' | 'download' | null>(null)
  const [prompt, setPrompt] = useState<PromptState | null>(null)

  const localSep: Sep = '\\'
  const remoteSep: Sep = '/'

  const loadLocal = useCallback((p: string) => localListDir(p), [])
  const loadRemote = useCallback((p: string) => sftpListDir(connection, p), [connection])

  const local = useDirPane(loadLocal)
  const remote = useDirPane(loadRemote)

  // Initial load — local home + remote home (empty path → backend resolves home).
  useEffect(() => {
    localHomeDir()
      .then((home) => local.navigate(home))
      .catch(() => local.navigate('C:\\'))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    remote.navigate('')
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connection])

  const enter = (pane: typeof local, entry: FileEntry) => {
    if (entry.is_dir) pane.navigate(entry.path)
  }

  // ── Mutations ──

  const makeDir = (
    pane: typeof local,
    sep: Sep,
    mkdir: (path: string) => Promise<void>,
  ) => {
    setPrompt({
      title: 'New folder name',
      initial: '',
      onConfirm: async (name) => {
        setPrompt(null)
        if (!name.trim()) return
        try {
          await mkdir(joinPath(pane.path, name.trim(), sep))
          await pane.navigate(pane.path)
        } catch (e) {
          addToast({ type: 'error', message: `mkdir failed: ${e}` })
        }
      },
    })
  }

  const renameEntry = (
    pane: typeof local,
    sep: Sep,
    entry: FileEntry,
    rename: (from: string, to: string) => Promise<void>,
  ) => {
    setPrompt({
      title: `Rename "${entry.name}"`,
      initial: entry.name,
      onConfirm: async (name) => {
        setPrompt(null)
        if (!name.trim() || name === entry.name) return
        try {
          await rename(entry.path, joinPath(parentPath(entry.path, sep), name.trim(), sep))
          await pane.navigate(pane.path)
        } catch (e) {
          addToast({ type: 'error', message: `rename failed: ${e}` })
        }
      },
    })
  }

  const deleteSelected = async (
    pane: typeof local,
    del: (entry: FileEntry) => Promise<void>,
  ) => {
    const targets = pane.entries.filter((e) => pane.selected.has(e.path))
    if (targets.length === 0) return
    setPrompt({
      title: `Delete ${targets.length} item(s)? This cannot be undone.`,
      initial: '',
      confirmLabel: 'Delete',
      destructive: true,
      hideInput: true,
      onConfirm: async () => {
        setPrompt(null)
        try {
          for (const t of targets) await del(t)
          await pane.navigate(pane.path)
        } catch (e) {
          addToast({ type: 'error', message: `delete failed: ${e}` })
        }
      },
    })
  }

  // ── Transfers ──

  const upload = async () => {
    const targets = local.entries.filter((e) => local.selected.has(e.path))
    if (targets.length === 0) return
    setBusy('upload')
    try {
      for (const t of targets) {
        const dest = joinPath(remote.path, baseName(t.path), remoteSep)
        await sftpUploadPath(connection, t.path, dest)
      }
      addToast({ type: 'success', message: `Uploaded ${targets.length} item(s)` })
      await remote.navigate(remote.path)
    } catch (e) {
      addToast({ type: 'error', message: `Upload failed: ${e}` })
    } finally {
      setBusy(null)
    }
  }

  const download = async () => {
    const targets = remote.entries.filter((e) => remote.selected.has(e.path))
    if (targets.length === 0) return
    setBusy('download')
    try {
      for (const t of targets) {
        const dest = joinPath(local.path, baseName(t.path), localSep)
        await sftpDownloadPath(connection, t.path, dest)
      }
      addToast({ type: 'success', message: `Downloaded ${targets.length} item(s)` })
      await local.navigate(local.path)
    } catch (e) {
      addToast({ type: 'error', message: `Download failed: ${e}` })
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="flex gap-3 flex-1 min-h-0 p-3">
      <FilePane
        title="Local"
        icon={<Monitor className="w-4 h-4 text-emerald-500" />}
        path={local.path}
        entries={local.entries}
        loading={local.loading}
        error={local.error}
        selected={local.selected}
        busy={busy === 'upload'}
        onEnter={(e) => enter(local, e)}
        onUp={() => local.navigate(parentPath(local.path, localSep))}
        onToggleSelect={local.toggleSelect}
        onRefresh={() => local.navigate(local.path)}
        onMkdir={() => makeDir(local, localSep, localMkdir)}
        onDelete={() => deleteSelected(local, (e) => localDelete(e.path))}
        onRename={(e) => renameEntry(local, localSep, e, localRename)}
        onNavigate={(p) => local.navigate(p)}
        transferLabel="Upload"
        transferIcon={<Upload className="w-3.5 h-3.5" />}
        onTransfer={upload}
      />

      <div className="flex flex-col items-center justify-center gap-2 shrink-0">
        <Upload className="w-4 h-4 text-content-muted" />
        <Download className="w-4 h-4 text-content-muted" />
      </div>

      <FilePane
        title={`Remote · ${connection}`}
        icon={<Server className="w-4 h-4 text-emerald-500" />}
        path={remote.path}
        entries={remote.entries}
        loading={remote.loading}
        error={remote.error}
        selected={remote.selected}
        busy={busy === 'download'}
        onEnter={(e) => enter(remote, e)}
        onUp={() => remote.navigate(parentPath(remote.path, remoteSep))}
        onToggleSelect={remote.toggleSelect}
        onRefresh={() => remote.navigate(remote.path)}
        onMkdir={() => makeDir(remote, remoteSep, (p) => sftpMkdir(connection, p))}
        onDelete={() =>
          deleteSelected(remote, (e) => sftpDelete(connection, e.path, e.is_dir))
        }
        onRename={(e) =>
          renameEntry(remote, remoteSep, e, (from, to) => sftpRename(connection, from, to))
        }
        onNavigate={(p) => remote.navigate(p)}
        transferLabel="Download"
        transferIcon={<Download className="w-3.5 h-3.5" />}
        onTransfer={download}
      />

      {prompt && <PromptModal state={prompt} onCancel={() => setPrompt(null)} />}
    </div>
  )
}
