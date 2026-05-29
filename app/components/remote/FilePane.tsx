import { useState, useEffect, type ReactNode } from 'react'
import {
  ArrowUp,
  RefreshCw,
  FolderPlus,
  Trash2,
  Pencil,
  Folder,
  File as FileIcon,
  Loader2,
} from 'lucide-react'
import clsx from 'clsx'
import type { FileEntry } from '../../lib/api'

interface FilePaneProps {
  title: string
  icon: ReactNode
  /** Current directory path (controlled). */
  path: string
  entries: FileEntry[]
  loading: boolean
  error: string | null
  /** Selected entry paths. */
  selected: Set<string>
  busy: boolean
  onEnter: (entry: FileEntry) => void
  onUp: () => void
  onToggleSelect: (entry: FileEntry, e: React.MouseEvent) => void
  onRefresh: () => void
  onMkdir: () => void
  onDelete: () => void
  onRename: (entry: FileEntry) => void
  onNavigate: (path: string) => void
  /** Transfer button (upload/download) — omitted if absent. */
  transferLabel?: string
  transferIcon?: ReactNode
  onTransfer?: () => void
}

function formatBytes(n: number): string {
  if (n === 0) return '—'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(n) / Math.log(1024))
  return `${(n / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`
}

function formatDate(secs: number): string {
  if (!secs) return ''
  try {
    return new Date(secs * 1000).toLocaleString()
  } catch {
    return ''
  }
}

export function FilePane({
  title,
  icon,
  path,
  entries,
  loading,
  error,
  selected,
  busy,
  onEnter,
  onUp,
  onToggleSelect,
  onRefresh,
  onMkdir,
  onDelete,
  onRename,
  onNavigate,
  transferLabel,
  transferIcon,
  onTransfer,
}: FilePaneProps) {
  const [pathInput, setPathInput] = useState(path)
  useEffect(() => setPathInput(path), [path])

  return (
    <div className="flex flex-col flex-1 min-w-0 bg-surface border border-edge rounded-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 bg-surface-raised border-b border-edge shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          {icon}
          <span className="text-sm font-medium text-content-secondary truncate">{title}</span>
        </div>
        <div className="flex items-center gap-0.5">
          <button
            onClick={onRefresh}
            className="p-1.5 text-content-muted hover:text-content hover:bg-hover rounded-md transition-colors"
            title="Refresh"
          >
            <RefreshCw className={clsx('w-3.5 h-3.5', loading && 'animate-spin')} />
          </button>
          <button
            onClick={onMkdir}
            className="p-1.5 text-content-muted hover:text-content hover:bg-hover rounded-md transition-colors"
            title="New folder"
          >
            <FolderPlus className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onDelete}
            disabled={selected.size === 0}
            className="p-1.5 text-content-muted hover:text-red-500 hover:bg-red-500/10 rounded-md transition-colors disabled:opacity-30 disabled:hover:bg-transparent"
            title="Delete selected"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Path bar */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-edge/50 bg-surface-alt shrink-0">
        <button
          onClick={onUp}
          className="p-1 text-content-muted hover:text-content hover:bg-hover rounded transition-colors shrink-0"
          title="Up one level"
        >
          <ArrowUp className="w-3.5 h-3.5" />
        </button>
        <form
          className="flex-1 min-w-0"
          onSubmit={(e) => {
            e.preventDefault()
            onNavigate(pathInput)
          }}
        >
          <input
            value={pathInput}
            onChange={(e) => setPathInput(e.target.value)}
            className="w-full bg-surface border border-edge rounded px-2 py-1 text-xs font-mono text-content-secondary focus:outline-none focus:border-emerald-500/50"
            spellCheck={false}
          />
        </form>
      </div>

      {/* File list */}
      <div className="flex-1 min-h-0 overflow-y-auto custom-scrollbar relative">
        {error && (
          <div className="m-2 p-2 text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded">
            {error}
          </div>
        )}
        {loading && entries.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center text-content-muted">
            <Loader2 className="w-5 h-5 animate-spin" />
          </div>
        )}
        {!loading && !error && entries.length === 0 && (
          <div className="absolute inset-0 flex items-center justify-center text-content-muted text-xs">
            Empty directory
          </div>
        )}
        <table className="w-full text-xs">
          <tbody>
            {entries.map((entry) => {
              const isSel = selected.has(entry.path)
              return (
                <tr
                  key={entry.path}
                  onClick={(e) => onToggleSelect(entry, e)}
                  onDoubleClick={() => onEnter(entry)}
                  className={clsx(
                    'cursor-pointer select-none border-b border-edge/20 group',
                    isSel ? 'bg-emerald-500/15' : 'hover:bg-hover',
                  )}
                >
                  <td className="py-1 pl-3 pr-2 w-5">
                    {entry.is_dir ? (
                      <Folder className="w-3.5 h-3.5 text-emerald-500/80 shrink-0" />
                    ) : (
                      <FileIcon className="w-3.5 h-3.5 text-content-muted shrink-0" />
                    )}
                  </td>
                  <td className="py-1 pr-2 text-content-secondary truncate max-w-0 w-full">
                    {entry.name}
                  </td>
                  <td className="py-1 pr-2 text-right text-content-muted whitespace-nowrap tabular-nums">
                    {entry.is_dir ? '' : formatBytes(entry.size)}
                  </td>
                  <td className="py-1 pr-2 text-right text-content-muted whitespace-nowrap hidden lg:table-cell">
                    {formatDate(entry.modified)}
                  </td>
                  <td className="py-1 pr-2 w-6">
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        onRename(entry)
                      }}
                      className="p-0.5 text-content-muted hover:text-content opacity-0 group-hover:opacity-100 transition-opacity"
                      title="Rename"
                    >
                      <Pencil className="w-3 h-3" />
                    </button>
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>

      {/* Footer / transfer */}
      <div className="flex items-center justify-between px-3 py-1.5 border-t border-edge bg-surface-raised shrink-0">
        <span className="text-[11px] text-content-muted">
          {selected.size > 0 ? `${selected.size} selected` : `${entries.length} items`}
        </span>
        {transferLabel && onTransfer && (
          <button
            onClick={onTransfer}
            disabled={selected.size === 0 || busy}
            className="flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium text-emerald-500 hover:bg-emerald-500/10 rounded-md transition-colors disabled:opacity-30 disabled:hover:bg-transparent"
          >
            {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : transferIcon}
            {transferLabel}
          </button>
        )}
      </div>
    </div>
  )
}
