import { useState, useEffect, useRef, useCallback } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { WebLinksAddon } from 'xterm-addon-web-links'
import { Unicode11Addon } from 'xterm-addon-unicode11'
import { listen } from '@tauri-apps/api/event'
import {
  spawnTerminal,
  writeTerminal,
  resizeTerminal,
  closeTerminal,
  getSites,
  generateAiContext,
  SiteWithStatus,
} from '../lib/api'
import { useApp } from '../lib/AppContext'
import { useTheme } from '../lib/ThemeContext'
import {
  Bot,
  Sparkles,
  Plus,
  X,
  Globe,
  FolderOpen,
  ChevronRight,
  ChevronLeft,
} from 'lucide-react'
import clsx from 'clsx'
import { XTERM_THEME_DARK, XTERM_THEME_LIGHT, XTERM_OPTIONS } from '../lib/xterm-config'

interface AiToolViewProps {
  tool: 'claude-code' | 'gemini-cli'
}

interface AiSession {
  id: string
  label: string
  domain: string
  tool: 'claude-code' | 'gemini-cli'
}

const MAX_AI_SESSIONS = 5

// Strip known document root suffixes to get the project root
function getProjectRoot(sitePath: string): string {
  const docRoots = ['public_html', 'public', 'dist', 'build', 'www', 'htdocs', 'web']
  const normalized = sitePath.replace(/[\\/]+$/, '')
  const lastSegment = normalized.split(/[\\/]/).pop()?.toLowerCase() || ''
  if (docRoots.includes(lastSegment)) {
    return normalized.substring(0, normalized.length - lastSegment.length - 1)
  }
  return normalized
}

export function AiToolView({ tool }: AiToolViewProps) {
  const { services, addToast } = useApp()
  const { resolvedTheme } = useTheme()

  const [sessions, setSessions] = useState<AiSession[]>([])
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null)
  const [sites, setSites] = useState<SiteWithStatus[]>([])
  const [showSiteSelector, setShowSiteSelector] = useState(false)
  const [infoPanelOpen, setInfoPanelOpen] = useState(() => {
    return localStorage.getItem('orbit-ai-info-panel') !== 'collapsed'
  })

  const nextSessionNum = useRef(1)
  const xtermsRef = useRef<Map<string, {
    term: XTerm
    fitAddon: FitAddon
    unlisten?: () => void
    ready: boolean
  }>>(new Map())
  const containersRef = useRef<Map<string, HTMLDivElement>>(new Map())
  const wrapperRef = useRef<HTMLDivElement>(null)

  const currentThemeRef = useRef(resolvedTheme)
  useEffect(() => { currentThemeRef.current = resolvedTheme }, [resolvedTheme])

  // Watch theme changes
  useEffect(() => {
    const themeToApply = resolvedTheme === 'light' ? XTERM_THEME_LIGHT : XTERM_THEME_DARK
    xtermsRef.current.forEach(({ term }) => { term.options.theme = themeToApply })
  }, [resolvedTheme])

  // Persist info panel state
  useEffect(() => {
    localStorage.setItem('orbit-ai-info-panel', infoPanelOpen ? 'open' : 'collapsed')
  }, [infoPanelOpen])

  // Load sites
  useEffect(() => {
    getSites().then(setSites).catch(() => setSites([]))
  }, [])

  const toolName = tool === 'claude-code' ? 'Claude Code' : 'Gemini CLI'
  const toolCmd = tool === 'claude-code' ? 'claude' : 'gemini'
  const toolIcon = tool === 'claude-code'
    ? <Bot size={16} className="text-orange-500" />
    : <Sparkles size={16} className="text-blue-500" />

  // Initialize xterm for a session
  const initXterm = useCallback(async (sessionId: string, cwd: string) => {
    const container = containersRef.current.get(sessionId)
    if (!container || xtermsRef.current.has(sessionId)) return

    const term = new XTerm({
      ...XTERM_OPTIONS,
      theme: currentThemeRef.current === 'light' ? XTERM_THEME_LIGHT : XTERM_THEME_DARK,
    })

    const fitAddon = new FitAddon()
    const unicode11 = new Unicode11Addon()
    term.loadAddon(fitAddon)
    term.loadAddon(new WebLinksAddon())
    term.loadAddon(unicode11)
    term.unicode.activeVersion = '11'
    term.open(container)
    fitAddon.fit()

    const entry = { term, fitAddon, unlisten: undefined as (() => void) | undefined, ready: false }
    xtermsRef.current.set(sessionId, entry)

    try {
      await spawnTerminal(sessionId, term.cols, term.rows, cwd)
      entry.ready = true

      term.onData((data: string) => {
        writeTerminal(sessionId, data).catch(console.error)
      })

      const unlisten = await listen(`pty-output-${sessionId}`, (event: { payload: string }) => {
        term.write(event.payload)
      })
      entry.unlisten = unlisten

      // Launch the AI CLI after shell is ready
      setTimeout(() => {
        writeTerminal(sessionId, `${toolCmd}\r`).catch(console.error)
      }, 800)
    } catch (err) {
      console.error('Failed to setup AI terminal:', err)
      term.write(`\r\n\x1b[31mFailed to start terminal: ${err}\x1b[0m\r\n`)
    }
  }, [toolCmd])

  // Create a new session for a site
  const createSession = useCallback(async (site: SiteWithStatus) => {
    if (sessions.length >= MAX_AI_SESSIONS) {
      addToast({ type: 'warning', message: `Maximum ${MAX_AI_SESSIONS} AI sessions` })
      return
    }

    // Check if already have a session for this site
    const existing = sessions.find(s => s.domain === site.domain)
    if (existing) {
      setActiveSessionId(existing.id)
      setShowSiteSelector(false)
      return
    }

    const num = nextSessionNum.current++
    const sessionId = `ai-${tool}-${num}`
    const projectRoot = getProjectRoot(site.path)

    // Generate context file (best effort)
    try {
      await generateAiContext(site.domain)
    } catch {
      // Non-critical, continue
    }

    const newSession: AiSession = {
      id: sessionId,
      label: site.domain.replace('.test', ''),
      domain: site.domain,
      tool,
    }

    setSessions(prev => [...prev, newSession])
    setActiveSessionId(sessionId)
    setShowSiteSelector(false)

    // Init xterm after render
    requestAnimationFrame(() => {
      initXterm(sessionId, projectRoot)
    })
  }, [sessions, tool, initXterm, addToast])

  // Close a session
  const closeSession = useCallback(async (sessionId: string) => {
    const entry = xtermsRef.current.get(sessionId)
    if (entry) {
      entry.unlisten?.()
      entry.term.dispose()
      xtermsRef.current.delete(sessionId)
    }
    containersRef.current.delete(sessionId)
    closeTerminal(sessionId).catch(console.error)

    setSessions(prev => {
      const remaining = prev.filter(s => s.id !== sessionId)
      setActiveSessionId(current => {
        if (current === sessionId) {
          const idx = prev.findIndex(s => s.id === sessionId)
          const nextIdx = idx > 0 ? idx - 1 : 0
          return remaining[nextIdx]?.id || null
        }
        return current
      })
      return remaining
    })
  }, [])

  // Fit active terminal on session switch
  useEffect(() => {
    if (!activeSessionId) return
    const entry = xtermsRef.current.get(activeSessionId)
    if (!entry?.ready) return

    const timer = setTimeout(() => {
      try {
        entry.fitAddon.fit()
        resizeTerminal(activeSessionId, entry.term.cols, entry.term.rows).catch(console.error)
      } catch { /* fit may fail */ }
    }, 50)
    return () => clearTimeout(timer)
  }, [activeSessionId, infoPanelOpen])

  // ResizeObserver
  useEffect(() => {
    if (!wrapperRef.current) return
    const observer = new ResizeObserver((entries) => {
      // Skip fit when hidden (zero dimensions)
      const rect = entries[0]?.contentRect
      if (!rect || rect.width === 0 || rect.height === 0) return

      if (!activeSessionId) return
      const entry = xtermsRef.current.get(activeSessionId)
      if (!entry?.ready) return
      try {
        entry.fitAddon.fit()
        resizeTerminal(activeSessionId, entry.term.cols, entry.term.rows).catch(console.error)
      } catch { /* fit may fail */ }
    })
    observer.observe(wrapperRef.current)
    return () => observer.disconnect()
  }, [activeSessionId])

  // Cleanup on unmount
  useEffect(() => {
    const xterms = xtermsRef.current
    return () => {
      xterms.forEach((entry, tabId) => {
        entry.unlisten?.()
        entry.term.dispose()
        closeTerminal(tabId).catch(console.error)
      })
      xterms.clear()
    }
  }, [])

  const setContainerRef = useCallback((sessionId: string, el: HTMLDivElement | null) => {
    if (el) containersRef.current.set(sessionId, el)
  }, [])

  // Active session info
  const activeSession = sessions.find(s => s.id === activeSessionId)
  const activeSite = activeSession ? sites.find(s => s.domain === activeSession.domain) : null

  // Running daemons for info panel
  const runningServices = services.filter(s =>
    ['nginx', 'apache', 'php', 'mariadb', 'postgresql', 'mongodb', 'redis', 'mailpit', 'meilisearch']
      .includes(s.service_type) && s.status === 'running'
  )

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 bg-surface-raised border-b border-edge shrink-0">
        {toolIcon}
        <span className="text-sm font-medium">{toolName}</span>
        <div className="w-px h-4 bg-edge mx-1" />

        {/* Session Tabs */}
        <div className="flex items-center gap-1 flex-1 overflow-x-auto" style={{ scrollbarWidth: 'none' }}>
          {sessions.map(session => (
            <div
              key={session.id}
              className={clsx(
                'flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium cursor-pointer transition-all group',
                activeSessionId === session.id
                  ? 'bg-emerald-500/10 text-emerald-500'
                  : 'text-content-muted hover:text-content-secondary hover:bg-hover'
              )}
              onClick={() => setActiveSessionId(session.id)}
            >
              <Globe className="w-3 h-3 shrink-0" />
              <span className="truncate max-w-[100px]">{session.label}</span>
              <button
                onClick={(e) => { e.stopPropagation(); closeSession(session.id) }}
                className="p-0.5 text-content-muted hover:text-red-500 hover:bg-red-500/10 rounded opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          ))}
        </div>

        {/* New session button — outside overflow container so dropdown isn't clipped */}
        {sessions.length < MAX_AI_SESSIONS && (
          <div className="relative shrink-0">
            <button
              onClick={() => setShowSiteSelector(!showSiteSelector)}
              className="p-1.5 text-content-muted hover:text-content hover:bg-hover rounded-md transition-colors"
              title="New session"
            >
              <Plus className="w-4 h-4" />
            </button>

            {/* Site selector dropdown */}
            {showSiteSelector && (
              <div className="absolute top-full right-0 mt-1 w-56 bg-surface-raised border border-edge rounded-lg shadow-xl z-50 py-1 max-h-64 overflow-y-auto">
                {sites.length === 0 ? (
                  <div className="px-3 py-2 text-xs text-content-muted">No sites found</div>
                ) : (
                  sites.map(site => (
                    <button
                      key={site.domain}
                      onClick={() => createSession(site)}
                      className="w-full flex items-center gap-2 px-3 py-2 text-xs text-content-secondary hover:bg-hover hover:text-content transition-colors"
                    >
                      <FolderOpen className="w-3.5 h-3.5 shrink-0 text-emerald-500" />
                      <span className="truncate">{site.domain}</span>
                      {sessions.some(s => s.domain === site.domain) && (
                        <span className="ml-auto text-[10px] text-emerald-500">open</span>
                      )}
                    </button>
                  ))
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Main Content */}
      <div className="flex flex-1 min-h-0 overflow-hidden" ref={wrapperRef}>
        {/* Terminal Area */}
        <div className="flex-1 min-h-0 relative bg-[#0d1117] dark:bg-[#0d1117] bg-[#f6f8fa]">
          {sessions.length === 0 && (
            <div className="absolute inset-0 flex flex-col items-center justify-center text-content-muted gap-4 bg-surface-alt">
              <div className="p-4 rounded-2xl bg-surface-raised border border-edge">
                {tool === 'claude-code'
                  ? <Bot className="w-12 h-12 text-orange-500/50" />
                  : <Sparkles className="w-12 h-12 text-blue-500/50" />
                }
              </div>
              <div className="text-center">
                <p className="text-sm font-medium text-content-secondary mb-1">
                  {toolName}
                </p>
                <p className="text-xs text-content-muted">
                  Select a project to start an AI session
                </p>
              </div>
              {sites.length > 0 && (
                <div className="flex flex-wrap gap-2 justify-center max-w-md">
                  {sites.slice(0, 6).map(site => (
                    <button
                      key={site.domain}
                      onClick={() => createSession(site)}
                      className="flex items-center gap-1.5 px-3 py-1.5 bg-surface-raised border border-edge hover:border-emerald-500/30 rounded-lg text-xs text-content-secondary hover:text-content transition-all"
                    >
                      <FolderOpen className="w-3 h-3 text-emerald-500" />
                      {site.domain.replace('.test', '')}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
          {sessions.map(session => (
            <div
              key={session.id}
              ref={el => setContainerRef(session.id, el)}
              className="absolute inset-0 p-1"
              style={{ display: activeSessionId === session.id ? 'block' : 'none' }}
            />
          ))}
        </div>

        {/* Info Panel */}
        {infoPanelOpen && activeSession && (
          <div className="w-56 border-l border-edge bg-surface-inset flex flex-col overflow-y-auto custom-scrollbar shrink-0">
            {/* Project */}
            {activeSite && (
              <div className="p-3 border-b border-edge/50">
                <div className="text-[10px] text-content-muted uppercase tracking-wider mb-2">Project</div>
                <div className="text-xs text-content-secondary space-y-1">
                  <div className="flex items-center gap-1.5">
                    <Globe className="w-3 h-3 text-emerald-500 shrink-0" />
                    <span className="truncate">{activeSite.domain}</span>
                  </div>
                  {activeSite.template && (
                    <div className="text-content-muted">Template: {activeSite.template}</div>
                  )}
                  {activeSite.php_version && (
                    <div className="text-content-muted">PHP: {activeSite.php_version}</div>
                  )}
                </div>
              </div>
            )}

            {/* Services */}
            {runningServices.length > 0 && (
              <div className="p-3 border-b border-edge/50">
                <div className="text-[10px] text-content-muted uppercase tracking-wider mb-2">Services</div>
                <div className="space-y-1">
                  {runningServices.map(svc => (
                    <div key={svc.name} className="flex items-center gap-1.5 text-xs text-content-secondary">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
                      <span>{svc.name}</span>
                      {svc.port && <span className="text-content-muted ml-auto">:{svc.port}</span>}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* MCP */}
            <div className="p-3 border-b border-edge/50">
              <div className="text-[10px] text-content-muted uppercase tracking-wider mb-2">MCP Tools</div>
              <div className="text-[11px] text-content-muted space-y-0.5">
                <p>orbit-mcp provides:</p>
                <p>- Service management</p>
                <p>- Database operations</p>
                <p>- Site management</p>
                <p>- Logs, SSL, PHP config</p>
              </div>
            </div>

            {/* Collapse button */}
            <div className="p-2 mt-auto">
              <button
                onClick={() => setInfoPanelOpen(false)}
                className="w-full flex items-center justify-center gap-1 px-2 py-1.5 text-[11px] text-content-muted hover:text-content-secondary hover:bg-hover rounded transition-colors"
              >
                <ChevronRight className="w-3 h-3" />
                Collapse
              </button>
            </div>
          </div>
        )}

        {/* Collapsed info panel toggle */}
        {!infoPanelOpen && activeSession && (
          <button
            onClick={() => setInfoPanelOpen(true)}
            className="w-8 shrink-0 border-l border-edge bg-surface-inset flex items-center justify-center hover:bg-hover transition-colors"
            title="Expand info panel"
          >
            <ChevronLeft className="w-4 h-4 text-content-muted" />
          </button>
        )}
      </div>
    </div>
  )
}
