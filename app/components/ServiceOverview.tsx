import { useEffect, useState } from 'react'
import { Power, RefreshCw, StopCircle, Download, AlertTriangle, ExternalLink } from 'lucide-react'
import { useApp, ServiceWithStatus } from '../lib/AppContext'
import { checkSystemRequirements } from '../lib/api'
import { getServiceIcon } from '../lib/serviceIcons'
import { open } from '@tauri-apps/plugin-shell'

interface ServiceOverviewProps {
  onNavigateToInstall: () => void
}

export function ServiceOverview({ onNavigateToInstall }: ServiceOverviewProps) {
  const {
    services,
    refreshServices,
    startServiceByName,
    stopServiceByName,
    startAllServices,
    stopAllServices,
  } = useApp()

  const [vcRedistMissing, setVcRedistMissing] = useState(false)

  useEffect(() => {
    checkSystemRequirements().then(reqs => {
      setVcRedistMissing(!reqs.vc_redist_installed)
    })
  }, [])

  const handleToggleService = async (service: ServiceWithStatus) => {
    if (service.status === 'running') {
      await stopServiceByName(service.name)
    } else if (service.status === 'stopped') {
      await startServiceByName(service.name)
    }
  }

  const runningCount = services.filter(s => s.status === 'running').length
  const totalCount = services.length

  return (
    <div>
      {/* Header */}
      <header className="mb-6 flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold mb-1">Overview</h2>
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
              onClick={onNavigateToInstall}
              className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-white transition-colors"
            >
              Install Services
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

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

  return (
    <div className="bg-surface-raised border border-edge-subtle rounded-xl p-4 hover:border-edge transition-all duration-300 group">
      <div className="flex justify-between items-start mb-3">
        <div className="flex items-center gap-3">
          <div className={`w-3 h-3 rounded-full transition-all duration-500 ${getStatusColor()}`} />
          <div className="flex items-center gap-2">
            <span className="text-xl">{getServiceIcon(service.service_type)}</span>
            <h3 className="font-semibold capitalize">{service.name}</h3>
          </div>
        </div>
        <button
          onClick={onToggle}
          disabled={isTransitioning}
          className={`px-3 py-1 rounded-md text-xs font-mono font-medium transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${isRunning
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
