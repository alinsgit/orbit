import { Activity, Server, Settings, Power, Download, RefreshCw, Minus, Square, X, StopCircle, ScrollText, AlertTriangle, ExternalLink, Database } from 'lucide-react'
import { useApp, ServiceWithStatus } from './lib/AppContext'
import { ServiceManager } from './components/ServiceManager'
import { SitesManager } from './components/SitesManager'
import { SettingsManager } from './components/SettingsManager'
import { LogViewer } from './components/LogViewer'
import DatabaseViewer from './components/DatabaseViewer'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { checkSystemRequirements } from './lib/api'
import { useEffect, useState } from 'react'
import { open } from '@tauri-apps/plugin-shell'

const appWindow = getCurrentWindow()

function App() {
  const {
    services,
    refreshServices,
    startServiceByName,
    stopServiceByName,
    startAllServices,
    stopAllServices,
    activeTab,
    setActiveTab
  } = useApp()

  const [vcRedistMissing, setVcRedistMissing] = useState(false)

  useEffect(() => {
    checkSystemRequirements().then(reqs => {
      setVcRedistMissing(!reqs.vc_redist_installed)
    })
  }, [])

  // Toggle service
  const handleToggleService = async (service: ServiceWithStatus) => {
    if (service.status === 'running') {
      await stopServiceByName(service.name)
    } else if (service.status === 'stopped') {
      await startServiceByName(service.name)
    }
  }

  // Running services count
  const runningCount = services.filter(s => s.status === 'running').length
  const totalCount = services.length

  return (
    <div className="app-container text-content font-sans select-none">
      {/* Title Bar */}
      <div className="h-11 bg-surface flex items-center px-4 border-b border-edge shrink-0" data-tauri-drag-region>
        <div className="flex items-center gap-2.5 pointer-events-none">
          {/* Logo */}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="12" cy="12" r="10" stroke="url(#orbit-gradient)" strokeWidth="2"/>
            <circle cx="12" cy="12" r="4" fill="url(#orbit-gradient)"/>
            <circle cx="12" cy="4" r="2" fill="#10b981"/>
            <defs>
              <linearGradient id="orbit-gradient" x1="4" y1="4" x2="20" y2="20" gradientUnits="userSpaceOnUse">
                <stop stopColor="#34d399"/>
                <stop offset="1" stopColor="#059669"/>
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
            active={activeTab === 'dashboard'}
            onClick={() => setActiveTab('dashboard')}
            icon={<Activity size={24} />}
            title="Dashboard"
          />
          <NavButton
            active={activeTab === 'services'}
            onClick={() => setActiveTab('services')}
            icon={<Download size={24} />}
            title="Services"
          />
          <NavButton
            active={activeTab === 'sites'}
            onClick={() => setActiveTab('sites')}
            icon={<Server size={24} />}
            title="Sites"
          />
          <NavButton
            active={activeTab === 'logs'}
            onClick={() => setActiveTab('logs')}
            icon={<ScrollText size={24} />}
            title="Logs"
          />
          <NavButton
            active={activeTab === 'database'}
            onClick={() => setActiveTab('database')}
            icon={<Database size={24} />}
            title="Database"
          />
          <div className="flex-1" />
          <NavButton
            active={activeTab === 'settings'}
            onClick={() => setActiveTab('settings')}
            icon={<Settings size={24} />}
            title="Settings"
          />
        </aside>

        {/* Main Content */}
        <main className="flex-1 min-h-0 bg-surface-alt overflow-hidden">
          {activeTab === 'dashboard' && (
            <div className="h-full overflow-y-auto p-6">
              {/* Header */}
              <header className="mb-6 flex justify-between items-center">
                <div>
                  <h1 className="text-2xl font-bold mb-1">Dashboard</h1>
                  <p className="text-content-secondary text-sm">
                    {runningCount}/{totalCount} services running
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={refreshServices}
                    className="p-2 bg-surface-raised hover:bg-hover rounded-lg transition-colors cursor-pointer"
                    title="Refresh"
                  >
                    <RefreshCw size={16} />
                  </button>
                  {runningCount > 0 && (
                    <button
                      onClick={stopAllServices}
                      className="flex items-center gap-2 px-4 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-500 rounded-lg text-sm font-medium transition-colors cursor-pointer"
                    >
                      <StopCircle size={16} />
                      Stop All
                    </button>
                  )}
                  <button
                    onClick={startAllServices}
                    className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors cursor-pointer shadow-lg shadow-emerald-900/20"
                  >
                    <Power size={16} />
                    Start All
                  </button>
                </div>
              </header>

              {/* VC++ Warning */}
              {vcRedistMissing && (
                <div className="mb-6 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl flex items-start gap-3">
                  <AlertTriangle className="text-amber-500 shrink-0 mt-0.5" size={20} />
                  <div>
                    <h3 className="font-semibold text-amber-500 mb-1">Visual C++ Redistributable Missing</h3>
                    <p className="text-sm text-content-secondary mb-3">
                      PHP and Nginx require the Visual C++ Redistributable 2015-2022 to run.
                      Services may fail to start without it.
                    </p>
                    <button
                      onClick={() => open('https://aka.ms/vs/17/release/vc_redist.x64.exe')}
                      className="flex items-center gap-2 px-3 py-1.5 bg-amber-500/20 hover:bg-amber-500/30 text-amber-500 rounded-lg text-sm font-medium transition-colors"
                    >
                      <Download size={14} />
                      Download Installer
                      <ExternalLink size={12} />
                    </button>
                  </div>
                </div>
              )}

              {/* Service Cards */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
                {services.map(service => (
                  <ServiceCard
                    key={service.name}
                    service={service}
                    onToggle={() => handleToggleService(service)}
                  />
                ))}
                {services.length === 0 && (
                  <div className="col-span-3 border-2 border-dashed border-edge rounded-xl p-12 text-center text-content-muted">
                    <Download size={48} className="mx-auto mb-4 opacity-50" />
                    <p className="mb-4">No services installed yet.</p>
                    <button
                      onClick={() => setActiveTab('services')}
                      className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-white transition-colors"
                    >
                      Install Services
                    </button>
                  </div>
                )}
              </div>

            </div>
          )}

          {activeTab === 'services' && <ServiceManager />}
          {activeTab === 'sites' && <SitesManager />}
          {activeTab === 'logs' && <LogViewer />}
          {activeTab === 'database' && <DatabaseViewer />}
          {activeTab === 'settings' && <SettingsManager />}
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
      className={`p-3 rounded-xl transition-all duration-200 cursor-pointer ${
        active
          ? 'bg-emerald-500/10 text-emerald-500 shadow-[0_0_15px_rgba(16,185,129,0.2)]'
          : 'text-content-muted hover:text-content-secondary hover:bg-hover'
      }`}
    >
      {icon}
    </button>
  )
}

// Service Card Component
function ServiceCard({ service, onToggle }: {
  service: ServiceWithStatus
  onToggle: () => void
}) {
  const isRunning = service.status === 'running'
  const isTransitioning = service.status === 'starting' || service.status === 'stopping'

  const getStatusColor = () => {
    switch (service.status) {
      case 'running': return 'bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]'
      case 'starting': return 'bg-amber-500 animate-pulse'
      case 'stopping': return 'bg-amber-500 animate-pulse'
      default: return 'bg-red-500'
    }
  }

  const getStatusText = () => {
    switch (service.status) {
      case 'running': return 'RUNNING'
      case 'starting': return 'STARTING...'
      case 'stopping': return 'STOPPING...'
      default: return 'STOPPED'
    }
  }

  const getServiceIcon = () => {
    const type = service.service_type.toLowerCase()
    if (type === 'nginx') return '🌐'
    if (type === 'apache') return '🪶'
    if (type === 'php') return '🐘'
    if (type === 'mariadb') return '🗄️'
    if (type === 'nodejs') return '💚'
    if (type === 'python') return '🐍'
    if (type === 'bun') return '🥟'
    return '⚙️'
  }

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4 hover:border-edge transition-all duration-300 group">
      <div className="flex justify-between items-start mb-3">
        <div className="flex items-center gap-3">
          <div className={`w-3 h-3 rounded-full transition-all duration-500 ${getStatusColor()}`} />
          <div className="flex items-center gap-2">
            <span className="text-xl">{getServiceIcon()}</span>
            <h3 className="font-semibold capitalize">{service.name}</h3>
          </div>
        </div>
        <button
          onClick={onToggle}
          disabled={isTransitioning}
          className={`px-3 py-1 rounded-md text-xs font-mono font-medium transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${
            isRunning
              ? 'bg-emerald-500/10 text-emerald-500 hover:bg-emerald-500/20'
              : 'bg-surface-inset text-content-secondary hover:bg-hover'
          }`}
        >
          {getStatusText()}
        </button>
      </div>
      <div className="flex justify-between text-sm text-content-secondary">
        <span>v{service.version}</span>
        <span className="font-mono text-content-muted">
          {service.port ? `:${service.port}` : '-'}
        </span>
      </div>
    </div>
  )
}

export default App
