import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import { Unicode11Addon } from 'xterm-addon-unicode11';
import { listen } from '@tauri-apps/api/event';
import { spawnTerminal, writeTerminal, resizeTerminal, closeTerminal, getWorkspacePath, getSites, SiteWithStatus } from '../lib/api';
import { useApp } from '../lib/AppContext';
import QUICK_COMMANDS from '../lib/terminal-commands.json';
import { Maximize2, Minimize2, Terminal as TerminalIcon, X, Plus, FolderOpen, Play, Square, RotateCw, Globe } from 'lucide-react';
import clsx from 'clsx';

interface TerminalProps {
  className?: string;
  onClose?: () => void;
}

interface TermTab {
  id: string;
  label: string;
  siteOrigin?: string; // site domain if opened from a site
  cwd?: string;
}

const MAX_TABS = 5;

const XTERM_THEME = {
  background: '#0a0a0a',
  foreground: '#f5f5f5',
  cursor: '#10b981',
  selectionBackground: 'rgba(16, 185, 129, 0.3)',
  black: '#000000',
  red: '#ef4444',
  green: '#10b981',
  yellow: '#f59e0b',
  blue: '#3b82f6',
  magenta: '#d946ef',
  cyan: '#06b6d4',
  white: '#ffffff',
  brightBlack: '#404040',
  brightRed: '#f87171',
  brightGreen: '#34d399',
  brightYellow: '#fbbf24',
  brightBlue: '#60a5fa',
  brightMagenta: '#e879f9',
  brightCyan: '#22d3ee',
  brightWhite: '#ffffff',
};

// Manageable service types
const MANAGEABLE_TYPES = ['nginx', 'php', 'mariadb', 'apache'];

// Strip known document root suffixes to get the project root
function getProjectRoot(sitePath: string): string {
  const docRoots = ['public_html', 'public', 'dist', 'build', 'www', 'htdocs', 'web'];
  const normalized = sitePath.replace(/[\\/]+$/, '');
  const lastSegment = normalized.split(/[\\/]/).pop()?.toLowerCase() || '';
  if (docRoots.includes(lastSegment)) {
    return normalized.substring(0, normalized.length - lastSegment.length - 1);
  }
  return normalized;
}

export function Terminal({ className, onClose }: TerminalProps) {
  const { services, startServiceByName, stopServiceByName, addToast, pendingTerminalSite, clearPendingTerminalSite } = useApp();

  // Tab state
  const [tabs, setTabs] = useState<TermTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string>('');
  const nextTabNum = useRef(1);

  // XTerm instances: tabId -> { term, fitAddon, unlisten, ready }
  const xtermsRef = useRef<Map<string, {
    term: XTerm;
    fitAddon: FitAddon;
    unlisten?: () => void;
    ready: boolean;
  }>>(new Map());

  // Container refs for each tab's div
  const containersRef = useRef<Map<string, HTMLDivElement>>(new Map());
  const wrapperRef = useRef<HTMLDivElement>(null);

  const [isExpanded, setIsExpanded] = useState(false);
  const [sites, setSites] = useState<SiteWithStatus[]>([]);

  // Load sites
  useEffect(() => {
    getSites().then(setSites).catch(() => setSites([]));
  }, []);

  // No auto-created tab — user opens tabs via "+" or site buttons

  // Handle pending terminal site from context
  useEffect(() => {
    if (!pendingTerminalSite) return;

    const { domain, path, command } = pendingTerminalSite;

    // Check if there's already a tab for this site
    const existingTab = tabs.find(t => t.siteOrigin === domain);
    if (existingTab) {
      setActiveTabId(existingTab.id);
      // If there's a command, run it in the existing tab
      if (command) {
        setTimeout(() => {
          writeTerminal(existingTab.id, `${command}\r`).catch(console.error);
        }, 500);
      }
      clearPendingTerminalSite();
      return;
    }

    // Create new tab for this site
    if (tabs.length >= MAX_TABS) {
      addToast({ type: 'warning', message: `Maximum ${MAX_TABS} terminals` });
      clearPendingTerminalSite();
      return;
    }

    const projectRoot = getProjectRoot(path);
    createTab(domain, domain, projectRoot, command);
    clearPendingTerminalSite();
  }, [pendingTerminalSite]);

  // Create a new tab
  const createTab = useCallback((label?: string, siteOrigin?: string, cwd?: string, initialCommand?: string) => {
    const num = nextTabNum.current++;
    const tabId = `tab-${num}`;
    const tabLabel = label || `Terminal ${num}`;

    const newTab: TermTab = { id: tabId, label: tabLabel, siteOrigin, cwd };
    setTabs(prev => [...prev, newTab]);
    setActiveTabId(tabId);

    // Initialize xterm after render
    requestAnimationFrame(() => {
      initXterm(tabId, cwd, initialCommand);
    });
  }, []);

  // Initialize xterm for a tab
  const initXterm = useCallback(async (tabId: string, cwd?: string, initialCommand?: string) => {
    const container = containersRef.current.get(tabId);
    if (!container || xtermsRef.current.has(tabId)) return;

    const term = new XTerm({
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Courier New', monospace",
      fontSize: 14,
      theme: XTERM_THEME,
      cursorBlink: true,
      cursorStyle: 'block',
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    const unicode11 = new Unicode11Addon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());
    term.loadAddon(unicode11);
    term.unicode.activeVersion = '11';
    term.open(container);
    fitAddon.fit();

    const entry = { term, fitAddon, unlisten: undefined as (() => void) | undefined, ready: false };
    xtermsRef.current.set(tabId, entry);

    try {
      const workspacePath = cwd || await getWorkspacePath() || undefined;
      await spawnTerminal(tabId, term.cols, term.rows, workspacePath);
      entry.ready = true;

      term.onData((data: string) => {
        writeTerminal(tabId, data).catch(console.error);
      });

      const unlisten = await listen(`pty-output-${tabId}`, (event: { payload: string }) => {
        term.write(event.payload);
      });
      entry.unlisten = unlisten;

      // Run initial command if provided (after shell has started)
      if (initialCommand) {
        setTimeout(() => {
          writeTerminal(tabId, `${initialCommand}\r`).catch(console.error);
        }, 800);
      }
    } catch (err) {
      console.error('Failed to setup terminal:', err);
      term.write(`\r\n\x1b[31mFailed to start terminal: ${err}\x1b[0m\r\n`);
    }
  }, []);

  // Close a tab
  const handleCloseTab = useCallback(async (tabId: string) => {
    const entry = xtermsRef.current.get(tabId);
    if (entry) {
      entry.unlisten?.();
      entry.term.dispose();
      xtermsRef.current.delete(tabId);
    }
    containersRef.current.delete(tabId);

    // Backend cleanup
    closeTerminal(tabId).catch(console.error);

    setTabs(prev => {
      const remaining = prev.filter(t => t.id !== tabId);

      // If closing last tab, close terminal panel
      if (remaining.length === 0) {
        onClose?.();
        return remaining;
      }

      // If closing active tab, switch to adjacent
      setActiveTabId(current => {
        if (current === tabId) {
          const idx = prev.findIndex(t => t.id === tabId);
          const nextIdx = idx > 0 ? idx - 1 : 0;
          return remaining[nextIdx]?.id || '';
        }
        return current;
      });

      return remaining;
    });
  }, [onClose]);

  // Handle site click -> open new tab
  const handleSiteClick = useCallback((site: SiteWithStatus) => {
    // Check if there's already a tab for this site
    const existingTab = tabs.find(t => t.siteOrigin === site.domain);
    if (existingTab) {
      setActiveTabId(existingTab.id);
      return;
    }

    if (tabs.length >= MAX_TABS) {
      addToast({ type: 'warning', message: `Maximum ${MAX_TABS} terminals` });
      return;
    }

    const projectRoot = getProjectRoot(site.path);
    createTab(site.domain, site.domain, projectRoot);
  }, [tabs, createTab, addToast]);

  // Fit active terminal on resize / tab switch / expand toggle
  useEffect(() => {
    const entry = xtermsRef.current.get(activeTabId);
    if (!entry?.ready) return;

    const timer = setTimeout(() => {
      try {
        entry.fitAddon.fit();
        resizeTerminal(activeTabId, entry.term.cols, entry.term.rows).catch(console.error);
      } catch { }
    }, 50);
    return () => clearTimeout(timer);
  }, [activeTabId, isExpanded]);

  // ResizeObserver for container size changes
  useEffect(() => {
    if (!wrapperRef.current) return;

    const observer = new ResizeObserver(() => {
      const entry = xtermsRef.current.get(activeTabId);
      if (!entry?.ready) return;
      try {
        entry.fitAddon.fit();
        resizeTerminal(activeTabId, entry.term.cols, entry.term.rows).catch(console.error);
      } catch { }
    });

    observer.observe(wrapperRef.current);
    return () => observer.disconnect();
  }, [activeTabId]);

  // Window resize handler
  useEffect(() => {
    const handleResize = () => {
      const entry = xtermsRef.current.get(activeTabId);
      if (!entry?.ready) return;
      try {
        entry.fitAddon.fit();
        resizeTerminal(activeTabId, entry.term.cols, entry.term.rows).catch(console.error);
      } catch { }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [activeTabId]);

  // Cleanup all on unmount
  useEffect(() => {
    return () => {
      xtermsRef.current.forEach((entry, tabId) => {
        entry.unlisten?.();
        entry.term.dispose();
        closeTerminal(tabId).catch(console.error);
      });
      xtermsRef.current.clear();
    };
  }, []);

  // Register container ref for a tab
  const setContainerRef = useCallback((tabId: string, el: HTMLDivElement | null) => {
    if (el) {
      containersRef.current.set(tabId, el);
    }
  }, []);

  const manageableServices = services.filter(s => MANAGEABLE_TYPES.includes(s.service_type));

  const handleRestart = useCallback(async (name: string) => {
    await stopServiceByName(name);
    await new Promise(r => setTimeout(r, 500));
    await startServiceByName(name);
  }, [stopServiceByName, startServiceByName]);

  const handleCommandPaste = (cmd: string) => {
    writeTerminal(activeTabId, cmd).catch(console.error);
  };

  const handleCommandRun = (cmd: string) => {
    writeTerminal(activeTabId, cmd + '\r').catch(console.error);
  };

  return (
    <div
      className={clsx(
        "flex flex-col bg-[#0a0a0a] border border-edge rounded-lg overflow-hidden glass-panel transition-all duration-300",
        isExpanded ? "fixed inset-4 z-50" : className
      )}
    >
      {/* Terminal Header */}
      <div className="flex items-center justify-between px-4 py-2 bg-surface-raised border-b border-edge">
        <div className="flex items-center gap-2">
          <TerminalIcon className="w-4 h-4 text-primary" />
          <span className="text-sm font-medium text-content-secondary">
            Orbit Terminal
          </span>
        </div>

        <div className="flex items-center gap-1">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="p-1.5 text-content-muted hover:text-content hover:bg-hover rounded-md transition-colors"
            title={isExpanded ? "Minimize" : "Maximize"}
          >
            {isExpanded ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
          </button>

          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 text-content-muted hover:text-error hover:bg-error/10 rounded-md transition-colors ml-1"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Tab Bar */}
      <div className="flex items-center bg-[#111] border-b border-edge/50 shrink-0">
        <div className="flex items-center flex-1 overflow-x-auto scrollbar-none" style={{ scrollbarWidth: 'none' }}>
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={clsx(
                "flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium whitespace-nowrap cursor-pointer transition-all border-r border-edge/30 group min-w-0",
                activeTabId === tab.id
                  ? "bg-[#0a0a0a] text-emerald-400"
                  : "text-content-muted hover:text-content-secondary hover:bg-white/5"
              )}
              onClick={() => setActiveTabId(tab.id)}
            >
              {tab.siteOrigin ? (
                <Globe className="w-3 h-3 shrink-0 text-emerald-500/70" />
              ) : (
                <TerminalIcon className="w-3 h-3 shrink-0" />
              )}
              <span className="truncate max-w-[120px]">{tab.label}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleCloseTab(tab.id);
                }}
                className="p-0.5 text-content-muted hover:text-red-400 hover:bg-red-500/10 rounded opacity-0 group-hover:opacity-100 transition-opacity ml-1 shrink-0"
                title="Close tab"
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          ))}

          {/* New tab button */}
          {tabs.length < MAX_TABS && (
            <button
              onClick={() => createTab()}
              className="p-1.5 text-content-muted hover:text-content hover:bg-white/5 transition-colors shrink-0"
              title="New terminal"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>
          )}
        </div>

        {/* Sites dropdown area */}
        {sites.length > 0 && (
          <div className="flex items-center gap-1 px-2 border-l border-edge/30 shrink-0">
            {sites.slice(0, 4).map((site) => (
              <button
                key={site.domain}
                onClick={() => handleSiteClick(site)}
                className={clsx(
                  "flex items-center gap-1 px-2 py-1 rounded text-[10px] font-medium whitespace-nowrap transition-all",
                  tabs.some(t => t.siteOrigin === site.domain)
                    ? "text-emerald-400 bg-emerald-500/10"
                    : "text-content-muted hover:text-content-secondary hover:bg-white/5"
                )}
                title={`Open terminal in ${site.domain}`}
              >
                <FolderOpen className="w-2.5 h-2.5" />
                {site.domain.replace('.test', '')}
              </button>
            ))}
            {sites.length > 4 && (
              <span className="text-[10px] text-content-muted px-1">+{sites.length - 4}</span>
            )}
          </div>
        )}
      </div>

      {/* Main Content Area */}
      <div className="flex flex-1 min-h-0 overflow-hidden" ref={wrapperRef}>
        {/* Terminal Containers — one per tab, only active is visible */}
        <div className="flex-1 min-h-0 relative">
          {tabs.length === 0 && (
            <div className="absolute inset-0 flex flex-col items-center justify-center text-content-muted gap-3">
              <TerminalIcon className="w-8 h-8 opacity-30" />
              <p className="text-sm">Click <span className="text-emerald-500 font-medium">+</span> or a site to open a terminal</p>
            </div>
          )}
          {tabs.map((tab) => (
            <div
              key={tab.id}
              ref={(el) => setContainerRef(tab.id, el)}
              className="absolute inset-0 p-2"
              style={{ display: activeTabId === tab.id ? 'block' : 'none' }}
            />
          ))}
        </div>

        {/* Quick Commands Sidebar */}
        <div className="w-64 border-l border-edge bg-surface-inset flex flex-col overflow-y-auto custom-scrollbar shadow-inner">
          <div className="p-3 border-b border-edge bg-surface-raised sticky top-0 z-10 font-semibold text-xs text-content-secondary uppercase tracking-wider">
            Quick Commands
          </div>

          <div className="p-2 flex flex-col gap-4">
            {/* Service Controls */}
            {manageableServices.length > 0 && (
              <div className="flex flex-col gap-1">
                <div className="text-[11px] text-content-muted uppercase tracking-wider pl-2 mb-1">
                  Service Controls
                </div>
                {manageableServices.map((svc) => (
                  <div key={svc.name} className="flex items-center justify-between rounded-md hover:bg-hover px-2 py-1 transition-colors">
                    <div className="flex items-center gap-1.5 flex-1 min-w-0">
                      <span className={clsx(
                        "w-1.5 h-1.5 rounded-full shrink-0",
                        svc.status === 'running' ? "bg-emerald-500" :
                          svc.status === 'starting' || svc.status === 'stopping' ? "bg-amber-500 animate-pulse" :
                            "bg-zinc-500"
                      )} />
                      <span className="text-xs text-content-secondary truncate">{svc.name}</span>
                    </div>
                    <div className="flex items-center gap-0.5 shrink-0">
                      {svc.status === 'stopped' ? (
                        <button
                          onClick={() => startServiceByName(svc.name)}
                          className="p-1 text-emerald-500 hover:text-emerald-400 hover:bg-emerald-500/10 rounded"
                          title={`Start ${svc.name}`}
                        >
                          <Play className="w-3 h-3" />
                        </button>
                      ) : svc.status === 'running' ? (
                        <>
                          <button
                            onClick={() => handleRestart(svc.name)}
                            className="p-1 text-amber-500 hover:text-amber-400 hover:bg-amber-500/10 rounded"
                            title={`Restart ${svc.name}`}
                          >
                            <RotateCw className="w-3 h-3" />
                          </button>
                          <button
                            onClick={() => stopServiceByName(svc.name)}
                            className="p-1 text-red-500 hover:text-red-400 hover:bg-red-500/10 rounded"
                            title={`Stop ${svc.name}`}
                          >
                            <Square className="w-3 h-3" />
                          </button>
                        </>
                      ) : (
                        <span className="text-[10px] text-amber-500 px-1">...</span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {QUICK_COMMANDS.map((group, groupIdx) => (
              <div key={groupIdx} className="flex flex-col gap-1">
                <div className="text-[11px] text-content-muted uppercase tracking-wider pl-2 mb-1">
                  {group.category}
                </div>
                {group.items.map((item, itemIdx) => (
                  <div key={itemIdx} className="flex items-center justify-between group rounded-md hover:bg-hover p-1.5 transition-colors">
                    <button
                      className="flex-1 text-left text-xs text-content-secondary group-hover:text-content truncate font-mono"
                      title={item.cmd}
                      onClick={() => handleCommandPaste(item.cmd)}
                    >
                      {item.label}
                    </button>
                    <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={() => handleCommandRun(item.cmd)}
                        className="p-1 text-emerald-500 hover:text-emerald-400 hover:bg-emerald-500/10 rounded"
                        title="Run instantly"
                      >
                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>
                      </button>
                      <button
                        onClick={() => handleCommandPaste(item.cmd)}
                        className="p-1 text-content-muted hover:text-content hover:bg-surface-raised rounded"
                        title="Paste"
                      >
                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
