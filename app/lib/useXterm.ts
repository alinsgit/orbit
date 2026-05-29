import { useCallback, useEffect, useRef } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { WebLinksAddon } from 'xterm-addon-web-links'
import { Unicode11Addon } from 'xterm-addon-unicode11'
import { listen } from '@tauri-apps/api/event'
import { useTheme } from './ThemeContext'
import { XTERM_THEME_DARK, XTERM_THEME_LIGHT, XTERM_OPTIONS } from './xterm-config'

interface UseXtermOptions {
  /** Stable session id. When null, no terminal is created. */
  id: string | null
  /** Spawn the backend PTY/shell for this id. */
  spawn: (id: string, cols: number, rows: number) => Promise<void>
  /** Send user input to the backend. */
  write: (id: string, data: string) => Promise<void>
  /** Notify the backend of a resize. */
  resize: (id: string, cols: number, rows: number) => Promise<void>
  /** Tear down the backend session. */
  close: (id: string) => Promise<void>
  /** Event name that streams output bytes for this id. */
  outputEvent: (id: string) => string
  /** Optional event fired when the backend session ends. */
  closedEvent?: (id: string) => string
  /** Called when the session closes (backend-initiated). */
  onClosed?: () => void
}

/**
 * Owns a single xterm.js instance wired to a backend PTY/SSH session.
 * Shared by the local Terminal and the remote SSH terminal — only the
 * spawn/write/resize/close + event-name callbacks differ.
 */
export function useXterm(opts: UseXtermOptions) {
  const { id } = opts
  const containerRef = useRef<HTMLDivElement | null>(null)
  const termRef = useRef<XTerm | null>(null)
  const fitRef = useRef<FitAddon | null>(null)
  const readyRef = useRef(false)

  const { resolvedTheme } = useTheme()
  // Keep callbacks/theme in refs so the create-effect runs once per id.
  const optsRef = useRef(opts)
  optsRef.current = opts
  const themeRef = useRef(resolvedTheme)
  themeRef.current = resolvedTheme

  // Live theme switching for the mounted terminal.
  useEffect(() => {
    if (termRef.current) {
      termRef.current.options.theme =
        resolvedTheme === 'light' ? XTERM_THEME_LIGHT : XTERM_THEME_DARK
    }
  }, [resolvedTheme])

  useEffect(() => {
    if (!id || !containerRef.current) return
    let disposed = false
    let unlisten: (() => void) | undefined
    let unlistenClosed: (() => void) | undefined

    const term = new XTerm({
      ...XTERM_OPTIONS,
      theme: themeRef.current === 'light' ? XTERM_THEME_LIGHT : XTERM_THEME_DARK,
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.loadAddon(new WebLinksAddon())
    const unicode11 = new Unicode11Addon()
    term.loadAddon(unicode11)
    term.unicode.activeVersion = '11'
    term.open(containerRef.current)
    fit.fit()

    termRef.current = term
    fitRef.current = fit

    const setup = async () => {
      try {
        await optsRef.current.spawn(id, term.cols, term.rows)
        if (disposed) return
        readyRef.current = true

        term.onData((data: string) => {
          optsRef.current.write(id, data).catch(console.error)
        })

        unlisten = await listen(
          optsRef.current.outputEvent(id),
          (event: { payload: string }) => {
            term.write(event.payload)
          },
        )

        if (optsRef.current.closedEvent) {
          unlistenClosed = await listen(optsRef.current.closedEvent(id), () => {
            term.write('\r\n\x1b[33m[session closed]\x1b[0m\r\n')
            readyRef.current = false
            optsRef.current.onClosed?.()
          })
        }
      } catch (err) {
        term.write(`\r\n\x1b[31mFailed to start session: ${err}\x1b[0m\r\n`)
      }
    }
    setup()

    return () => {
      disposed = true
      readyRef.current = false
      unlisten?.()
      unlistenClosed?.()
      term.dispose()
      termRef.current = null
      fitRef.current = null
      optsRef.current.close(id).catch(console.error)
    }
  }, [id])

  /** Refit to the container and push the new size to the backend. */
  const refit = useCallback(() => {
    const term = termRef.current
    const fit = fitRef.current
    if (!term || !fit || !readyRef.current || !id) return
    try {
      fit.fit()
      optsRef.current.resize(id, term.cols, term.rows).catch(console.error)
    } catch {
      /* fit may fail when the container is hidden */
    }
  }, [id])

  // Refit on container resize.
  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect
      if (!rect || rect.width === 0 || rect.height === 0) return
      refit()
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [refit])

  const focus = () => termRef.current?.focus()

  return { containerRef, refit, focus }
}
