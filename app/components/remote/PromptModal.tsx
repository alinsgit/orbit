import { useState, useEffect, useRef } from 'react'
import clsx from 'clsx'

export interface PromptState {
  title: string
  initial: string
  confirmLabel?: string
  destructive?: boolean
  /** Render as a confirmation dialog with no text input. */
  hideInput?: boolean
  onConfirm: (value: string) => void
}

interface PromptModalProps {
  state: PromptState
  onCancel: () => void
}

export function PromptModal({ state, onCancel }: PromptModalProps) {
  const [value, setValue] = useState(state.initial)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setValue(state.initial)
    if (!state.hideInput) {
      // Focus + select on open.
      const t = setTimeout(() => inputRef.current?.select(), 30)
      return () => clearTimeout(t)
    }
  }, [state])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onCancel])

  return (
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/40 backdrop-blur-sm"
      onMouseDown={onCancel}
    >
      <div
        className="w-[380px] bg-surface border border-edge rounded-lg shadow-xl overflow-hidden"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="px-4 py-3 border-b border-edge">
          <h3 className="text-sm font-medium text-content-secondary">{state.title}</h3>
        </div>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            state.onConfirm(value)
          }}
        >
          {!state.hideInput && (
            <div className="px-4 py-3">
              <input
                ref={inputRef}
                value={value}
                onChange={(e) => setValue(e.target.value)}
                autoFocus
                spellCheck={false}
                className="w-full bg-surface-alt border border-edge rounded px-2.5 py-1.5 text-sm text-content focus:outline-none focus:border-emerald-500/50"
              />
            </div>
          )}
          <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-edge bg-surface-raised">
            <button
              type="button"
              onClick={onCancel}
              className="px-3 py-1.5 text-xs font-medium text-content-muted hover:text-content hover:bg-hover rounded-md transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              className={clsx(
                'px-3 py-1.5 text-xs font-medium rounded-md transition-colors',
                state.destructive
                  ? 'text-white bg-red-500 hover:bg-red-600'
                  : 'text-white bg-emerald-500 hover:bg-emerald-600',
              )}
            >
              {state.confirmLabel || 'OK'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
