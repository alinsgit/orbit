import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import { listen, Event } from '@tauri-apps/api/event';
import { spawnTerminal, writeTerminal, resizeTerminal, getWorkspacePath, getSites, SiteWithStatus } from '../lib/api';
import { useApp } from '../lib/AppContext';
import QUICK_COMMANDS from '../lib/terminal-commands.json';
import { Maximize2, Minimize2, Terminal as TerminalIcon, X, ChevronLeft, ChevronRight, FolderOpen, Play, Square, RotateCw } from 'lucide-react';
import clsx from 'clsx';

interface TerminalProps {
  id?: string;
  className?: string;
  onClose?: () => void;
}

// Manageable service types (ones that can be started/stopped)
const MANAGEABLE_TYPES = ['nginx', 'php', 'mariadb', 'apache'];

export function Terminal({ id = 'main-term', className, onClose }: TerminalProps) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [isReady, setIsReady] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [sites, setSites] = useState<SiteWithStatus[]>([]);
  const [activeSite, setActiveSite] = useState<string | null>(null);
  const sliderRef = useRef<HTMLDivElement>(null);
  const { services, startServiceByName, stopServiceByName } = useApp();

  useEffect(() => {
    if (!terminalRef.current) return;

    // Initialize xterm.js
    const term = new XTerm({
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Courier New', monospace",
      fontSize: 14,
      theme: {
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
      },
      cursorBlink: true,
      cursorStyle: 'block',
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    term.open(terminalRef.current);
    fitAddon.fit();

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    let unlisten: (() => void) | undefined;
    let initialSpawnDone = false;

    const setupTerminal = async () => {
      try {
        const workspacePath = await getWorkspacePath();
        // Spawn the native PTY process
        await spawnTerminal(id, term.cols, term.rows, workspacePath || undefined);
        setIsReady(true);
        initialSpawnDone = true;

        // Listen for user keyboard input and send to PTY
        term.onData((data: string) => {
          writeTerminal(id, data).catch(console.error);
        });

        // Listen for PTY output and write to xterm
        unlisten = await listen(`pty-output-${id}`, (event: Event<string>) => {
          term.write(event.payload);
        });
      } catch (err) {
        console.error('Failed to setup terminal:', err);
        term.write(`\r\n\x1b[31mFailed to start terminal: ${err}\x1b[0m\r\n`);
      }
    };

    setupTerminal();

    // Handle window resizing
    const handleResize = () => {
      if (fitAddon && term && initialSpawnDone) {
        try {
          fitAddon.fit();
          resizeTerminal(id, term.cols, term.rows).catch(console.error);
        } catch (e) {
          console.error("Resize error:", e);
        }
      }
    };

    window.addEventListener('resize', handleResize);

    // Fit again after a short delay to ensure fonts/layout are loaded
    setTimeout(handleResize, 100);

    // Watch container size changes (e.g. sites bar loading, layout shifts)
    let resizeObserver: ResizeObserver | undefined;
    if (terminalRef.current) {
      resizeObserver = new ResizeObserver(() => handleResize());
      resizeObserver.observe(terminalRef.current);
    }

    return () => {
      window.removeEventListener('resize', handleResize);
      resizeObserver?.disconnect();
      if (unlisten) unlisten();
      term.dispose();
    };
  }, [id]);

  // Effect to handle manual expand toggling resize
  useEffect(() => {
    if (isReady && fitAddonRef.current && xtermRef.current) {
      setTimeout(() => {
        try {
          fitAddonRef.current?.fit();
          resizeTerminal(id, xtermRef.current!.cols, xtermRef.current!.rows).catch(console.error);
        } catch(e) {}
      }, 50); // Small delay to let CSS transition finish
    }
  }, [isExpanded, isReady, id]);

  // Load sites
  useEffect(() => {
    getSites().then(setSites).catch(() => setSites([]));
  }, []);

  // Strip known document root suffixes to get the project root
  const getProjectRoot = (sitePath: string): string => {
    const docRoots = ['public_html', 'public', 'dist', 'build', 'www', 'htdocs', 'web'];
    const normalized = sitePath.replace(/[\\/]+$/, ''); // trim trailing slashes
    const lastSegment = normalized.split(/[\\/]/).pop()?.toLowerCase() || '';
    if (docRoots.includes(lastSegment)) {
      return normalized.substring(0, normalized.length - lastSegment.length - 1);
    }
    return normalized;
  };

  const handleSiteClick = useCallback((site: SiteWithStatus) => {
    if (!isReady) return;
    setActiveSite(site.domain);
    const projectRoot = getProjectRoot(site.path);
    writeTerminal(id, `cd "${projectRoot}"\r`).catch(console.error);
  }, [id, isReady]);

  const scrollSlider = useCallback((dir: 'left' | 'right') => {
    if (!sliderRef.current) return;
    sliderRef.current.scrollBy({ left: dir === 'left' ? -200 : 200, behavior: 'smooth' });
  }, []);

  const manageableServices = services.filter(s => MANAGEABLE_TYPES.includes(s.service_type));

  const handleRestart = useCallback(async (name: string) => {
    await stopServiceByName(name);
    // Small delay to ensure process is fully stopped
    await new Promise(r => setTimeout(r, 500));
    await startServiceByName(name);
  }, [stopServiceByName, startServiceByName]);

  const handleCommandPaste = (cmd: string) => {
    writeTerminal(id, cmd).catch(console.error);
  };

  const handleCommandRun = (cmd: string) => {
    writeTerminal(id, cmd + '\r').catch(console.error);
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
          {!isReady && (
            <span className="flex h-2 w-2 relative ml-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-warning opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-warning"></span>
            </span>
          )}
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

      {/* Sites Slider Bar */}
      {sites.length > 0 && (
        <div className="flex items-center gap-0 bg-[#111] border-b border-edge/50 shrink-0">
          <button
            onClick={() => scrollSlider('left')}
            className="p-1 text-content-muted hover:text-content shrink-0 hover:bg-white/5 transition-colors"
          >
            <ChevronLeft className="w-3.5 h-3.5" />
          </button>
          <div
            ref={sliderRef}
            className="flex items-center gap-1.5 overflow-x-auto py-1.5 px-1 flex-1 scrollbar-none"
            style={{ scrollbarWidth: 'none' }}
          >
            {sites.map((site) => (
              <button
                key={site.domain}
                onClick={() => handleSiteClick(site)}
                className={clsx(
                  "flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] font-medium whitespace-nowrap transition-all shrink-0",
                  activeSite === site.domain
                    ? "bg-emerald-500/15 text-emerald-400 ring-1 ring-emerald-500/30"
                    : "text-content-muted hover:text-content-secondary hover:bg-white/5"
                )}
                title={site.path}
              >
                <FolderOpen className="w-3 h-3 shrink-0" />
                {site.domain}
              </button>
            ))}
          </div>
          <button
            onClick={() => scrollSlider('right')}
            className="p-1 text-content-muted hover:text-content shrink-0 hover:bg-white/5 transition-colors"
          >
            <ChevronRight className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {/* Main Content Area */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {/* Terminal Container */}
        <div
          className="flex-1 min-h-0 p-2 overflow-hidden"
          ref={terminalRef}
        />

        {/* Quick Commands Sidebar */}
        <div className="w-64 border-l border-edge bg-surface-inset flex flex-col overflow-y-auto custom-scrollbar shadow-inner">
          <div className="p-3 border-b border-edge bg-surface-raised sticky top-0 z-10 font-semibold text-xs text-content-secondary uppercase tracking-wider">
            Quick Commands
          </div>

          <div className="p-2 flex flex-col gap-4">
            {/* Service Controls — direct API, no CLI dependency */}
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
