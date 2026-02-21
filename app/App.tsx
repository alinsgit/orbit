import { Server, Settings, Minus, Square, X, ScrollText, Database, Globe, TerminalSquare, Loader2 } from 'lucide-react'
import { useApp } from './lib/AppContext'
import { ServiceManager } from './components/ServiceManager'
import { SitesManager } from './components/SitesManager'
import { SettingsManager } from './components/SettingsManager'
import { LogViewer } from './components/LogViewer'
import DatabaseViewer from './components/DatabaseViewer'
import { Terminal } from './components/Terminal'
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

function App() {
  const {
    activeTab,
    setActiveTab,
    isTerminalOpen,
    setIsTerminalOpen,
    isLoading
  } = useApp()

  return (
    <div className="app-container text-content font-sans select-none relative">
      {/* Global Loading Overlay */}
      {isLoading && (
        <div className="absolute inset-0 z-[100] bg-black/40 backdrop-blur-sm flex flex-col items-center justify-center animate-in fade-in duration-200">
          <div className="bg-surface border border-edge shadow-2xl rounded-2xl p-6 flex flex-col items-center gap-4 animate-in zoom-in-95 duration-200">
            <Loader2 size={32} className="animate-spin text-emerald-500" />
            <p className="text-sm font-medium text-content-secondary">Waking up services...</p>
          </div>
        </div>
      )}

      {/* Title Bar */}
      <div className="h-11 bg-surface flex items-center px-4 border-b border-edge shrink-0" data-tauri-drag-region>
        <div className="flex items-center gap-2.5 pointer-events-none">
          {/* Logo */}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="12" cy="12" r="10" stroke="url(#orbit-gradient)" strokeWidth="2" />
            <circle cx="12" cy="12" r="4" fill="url(#orbit-gradient)" />
            <circle cx="12" cy="4" r="2" fill="#10b981" />
            <defs>
              <linearGradient id="orbit-gradient" x1="4" y1="4" x2="20" y2="20" gradientUnits="userSpaceOnUse">
                <stop stopColor="#34d399" />
                <stop offset="1" stopColor="#059669" />
              </linearGradient>
            </defs>
          </svg>
          <div className="flex items-baseline gap-1.5">
            <span className="font-semibold text-sm text-content">Orbit</span>
            <span className="text-[10px] text-content-muted font-medium uppercase tracking-wider">Dev Environment</span>
          </div>
        </div>
        <div className="flex-1 h-full" data-tauri-drag-region />
        <div className="flex gap-1 items-center">
          <button
            onClick={() => appWindow.minimize()}
            className="p-1.5 hover:bg-hover rounded transition-colors cursor-pointer"
            title="Minimize"
          >
            <Minus size={14} className="text-content-secondary" />
          </button>
          <button
            onClick={() => appWindow.toggleMaximize()}
            className="p-1.5 hover:bg-hover rounded transition-colors cursor-pointer"
            title="Maximize"
          >
            <Square size={12} className="text-content-secondary" />
          </button>
          <button
            onClick={() => appWindow.close()}
            className="p-1.5 hover:bg-red-500/20 hover:text-red-500 rounded transition-colors cursor-pointer"
            title="Close"
          >
            <X size={14} className="text-content-muted hover:text-red-500" />
          </button>
        </div>
      </div>

      <div className="flex flex-1 min-h-0">
        {/* Sidebar */}
        <aside className="w-16 shrink-0 bg-surface flex flex-col items-center py-4 gap-4 border-r border-edge">
          <NavButton
            active={activeTab === 'services'}
            onClick={() => setActiveTab('services')}
            icon={<Server size={24} />}
            title="Services"
          />
          <NavButton
            active={activeTab === 'sites'}
            onClick={() => setActiveTab('sites')}
            icon={<Globe size={24} />}
            title="Sites"
          />
          <NavButton
            active={activeTab === 'database'}
            onClick={() => setActiveTab('database')}
            icon={<Database size={24} />}
            title="Database"
          />
          <NavButton
            active={activeTab === 'logs'}
            onClick={() => setActiveTab('logs')}
            icon={<ScrollText size={24} />}
            title="Logs"
          />
          <div className="flex-1" />
          <NavButton
            active={isTerminalOpen}
            onClick={() => setIsTerminalOpen(!isTerminalOpen)}
            icon={<TerminalSquare size={24} />}
            title="Terminal"
          />
          <NavButton
            active={activeTab === 'settings'}
            onClick={() => { setActiveTab('settings'); setIsTerminalOpen(false); }}
            icon={<Settings size={24} />}
            title="Settings"
          />
        </aside>

        {/* Main Content */}
        <main className="flex-1 min-h-0 relative flex flex-col bg-surface-alt overflow-hidden">
          <div className="flex-1 min-h-0 overflow-auto">
            {activeTab === 'services' && <ServiceManager />}
            {activeTab === 'sites' && <SitesManager />}
            {activeTab === 'logs' && <LogViewer />}
            {activeTab === 'database' && <DatabaseViewer />}
            {activeTab === 'settings' && <SettingsManager />}
          </div>
          
          {/* Docked Terminal */}
          {isTerminalOpen && (
             <div className="h-2/5 min-h-[250px] border-t border-edge bg-[#0a0a0a] relative z-40 flex flex-col">
                <Terminal onClose={() => setIsTerminalOpen(false)} className="w-full h-full border-0 rounded-none" />
             </div>
          )}
        </main>
      </div>
    </div>
  )
}

// Navigation Button Component
function NavButton({ active, onClick, icon, title }: {
  active: boolean
  onClick: () => void
  icon: React.ReactNode
  title: string
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className={`p-3 rounded-xl transition-all duration-200 cursor-pointer ${active
        ? 'bg-emerald-500/10 text-emerald-500 shadow-[0_0_15px_rgba(16,185,129,0.2)]'
        : 'text-content-muted hover:text-content-secondary hover:bg-hover'
        }`}
    >
      {icon}
    </button>
  )
}

export default App
