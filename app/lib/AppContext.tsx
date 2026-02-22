import { createContext, useContext, useState, useEffect, useRef, ReactNode, useCallback } from 'react';
import {
  getInstalledServices,
  getServiceStatus,
  startService,
  stopService,
  autoStartServices,
  InstalledService
} from './api';
import { settingsStore, Settings } from './store';

// Service with runtime status
export interface ServiceWithStatus extends InstalledService {
  status: 'running' | 'stopped' | 'starting' | 'stopping';
  port?: number;
}

// Toast notification
export interface Toast {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  duration?: number;
}

interface AppContextType {
  // Services
  services: ServiceWithStatus[];
  refreshServices: (showLoader?: boolean) => Promise<void>;
  startServiceByName: (name: string) => Promise<void>;
  stopServiceByName: (name: string) => Promise<void>;
  startAllServices: () => Promise<void>;
  stopAllServices: () => Promise<void>;
  getInstalledPhpVersions: () => string[];

  // Settings
  settings: Settings;
  refreshSettings: () => Promise<void>;

  // Loading state
  isLoading: boolean;
  setLoading: (loading: boolean) => void;

  // Toast notifications
  toasts: Toast[];
  addToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;

  // Active tab (for cross-component communication)
  activeTab: string;
  setActiveTab: (tab: string) => void;

  // Terminal state
  isTerminalOpen: boolean;
  setIsTerminalOpen: (open: boolean) => void;
}

const AppContext = createContext<AppContextType | undefined>(undefined);

export function AppProvider({ children }: { children: ReactNode }) {
  const [services, setServices] = useState<ServiceWithStatus[]>([]);
  const [settings, setSettings] = useState<Settings>({
    ports: { nginx: 80, php_start: 9000, mariadb: 3306 },
    paths: { bin_dir: '', data_dir: '' },
    services: { auto_start: false, autostart_list: [], default_php: '8.3' }
  });
  const [isLoading, setIsLoading] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [activeTab, setActiveTab] = useState('services');
  const [isTerminalOpen, setIsTerminalOpen] = useState(false);

  // Generate unique ID for toasts
  const generateId = () => Math.random().toString(36).substring(2, 9);

  // Add toast notification
  const addToast = useCallback((toast: Omit<Toast, 'id'>) => {
    const id = generateId();
    const newToast = { ...toast, id };
    setToasts(prev => [...prev, newToast]);

    // Auto-remove after duration
    const duration = toast.duration || 5000;
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, duration);
  }, []);

  // Remove toast
  const removeToast = useCallback((id: string) => {
    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  // Refresh settings
  const refreshSettings = useCallback(async () => {
    try {
      const s = await settingsStore.getSettings();
      setSettings(s);
    } catch (e) {
      console.error('Failed to load settings:', e);
    }
  }, []);

  // Refresh services with status
  // showLoader=true only on first load; subsequent refreshes are silent
  const refreshServices = useCallback(async (showLoader = false) => {
    if (showLoader) setIsLoading(true);
    try {
      const installed = await getInstalledServices();

      const withStatus: ServiceWithStatus[] = await Promise.all(
        installed.map(async (service) => {
          let status: 'running' | 'stopped' = 'stopped';
          try {
            const s = await getServiceStatus(service.name);
            status = s as 'running' | 'stopped';
          } catch {
            status = 'stopped';
          }

          return { ...service, status, port: service.port };
        })
      );

      setServices(withStatus);
    } catch (e) {
      console.error('Failed to refresh services:', e);
      addToast({ type: 'error', message: 'Failed to load services' });
    } finally {
      if (showLoader) setIsLoading(false);
    }
  }, [addToast]);

  // Start service by name
  const startServiceByName = useCallback(async (name: string) => {
    const service = services.find(s => s.name === name);
    if (!service) {
      addToast({ type: 'error', message: `Service ${name} not found` });
      return;
    }

    // Update status to starting
    setServices(prev => prev.map(s =>
      s.name === name ? { ...s, status: 'starting' as const } : s
    ));

    try {
      await startService(name, service.path);
      setServices(prev => prev.map(s =>
        s.name === name ? { ...s, status: 'running' as const } : s
      ));
      addToast({ type: 'success', message: `${name} started successfully` });
    } catch (e: any) {
      setServices(prev => prev.map(s =>
        s.name === name ? { ...s, status: 'stopped' as const } : s
      ));
      addToast({ type: 'error', message: `Failed to start ${name}: ${e}` });
    }
  }, [services, addToast]);

  // Stop service by name
  const stopServiceByName = useCallback(async (name: string) => {
    // Update status to stopping
    setServices(prev => prev.map(s =>
      s.name === name ? { ...s, status: 'stopping' as const } : s
    ));

    try {
      await stopService(name);
      setServices(prev => prev.map(s =>
        s.name === name ? { ...s, status: 'stopped' as const } : s
      ));
      addToast({ type: 'success', message: `${name} stopped` });
    } catch (e: any) {
      // Refresh to get actual status
      await refreshServices();
      addToast({ type: 'error', message: `Failed to stop ${name}: ${e}` });
    }
  }, [addToast, refreshServices]);

  // Start all services
  const startAllServices = useCallback(async () => {
    setIsLoading(true);
    for (const service of services) {
      if (service.status === 'stopped') {
        await startServiceByName(service.name);
      }
    }
    setIsLoading(false);
  }, [services, startServiceByName]);

  // Stop all services
  const stopAllServices = useCallback(async () => {
    setIsLoading(true);
    for (const service of services) {
      if (service.status === 'running') {
        await stopServiceByName(service.name);
      }
    }
    setIsLoading(false);
  }, [services, stopServiceByName]);

  // Get installed PHP versions
  const getInstalledPhpVersions = useCallback(() => {
    return services
      .filter(s => s.service_type === 'php')
      .map(s => s.version);
  }, [services]);

  // Set loading state
  const setLoading = useCallback((loading: boolean) => {
    setIsLoading(loading);
  }, []);

  // Initial load
  useEffect(() => {
    refreshSettings();
  }, [refreshSettings]);

  // Load services after settings (first load — show loader)
  const initialLoadDone = useRef(false);
  const autostartRan = useRef(false);
  useEffect(() => {
    refreshServices(!initialLoadDone.current).then(() => {
      initialLoadDone.current = true;
    });
  }, [refreshServices]);

  // Autostart services on first load
  useEffect(() => {
    if (!initialLoadDone.current || autostartRan.current) return;
    if (services.length === 0) return;

    const list = settings.services.autostart_list;
    if (!list || list.length === 0) return;

    // Only autostart services that are currently stopped
    const toStart = list.filter(name =>
      services.some(s => s.name === name && s.status === 'stopped')
    );
    if (toStart.length === 0) return;

    autostartRan.current = true;
    autoStartServices(toStart).then(() => {
      addToast({ type: 'info', message: `Autostarted ${toStart.length} service(s)` });
      // Refresh to get updated statuses
      refreshServices();
    }).catch(err => {
      console.error('Autostart failed:', err);
    });
  }, [services, settings]);

  // Refresh services when switching to dashboard tab (silent)
  useEffect(() => {
    if (activeTab === 'services') {
      refreshServices();
    }
  }, [activeTab]);

  return (
    <AppContext.Provider value={{
      services,
      refreshServices,
      startServiceByName,
      stopServiceByName,
      startAllServices,
      stopAllServices,
      getInstalledPhpVersions,
      settings,
      refreshSettings,
      isLoading,
      setLoading,
      toasts,
      addToast,
      removeToast,
      activeTab,
      setActiveTab,
      isTerminalOpen,
      setIsTerminalOpen,
    }}>
      {children}

      {/* Global Loading Overlay */}
      {isLoading && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-surface-raised rounded-xl p-6 flex items-center gap-3">
            <div className="w-6 h-6 border-2 border-emerald-500 border-t-transparent rounded-full animate-spin" />
            <span className="text-content">Processing...</span>
          </div>
        </div>
      )}

      {/* Toast Notifications */}
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
        {toasts.map(toast => (
          <div
            key={toast.id}
            className={`px-4 py-3 rounded-lg shadow-lg max-w-sm flex items-start gap-3 animate-slide-in ${
              toast.type === 'success' ? 'bg-emerald-600 text-white' :
              toast.type === 'error' ? 'bg-red-600 text-white' :
              toast.type === 'warning' ? 'bg-amber-600 text-white' :
              'bg-blue-600 text-white'
            }`}
          >
            <span className="flex-1 text-sm">{toast.message}</span>
            <button
              onClick={() => removeToast(toast.id)}
              className="text-white/80 hover:text-white text-lg leading-none"
            >
              ×
            </button>
          </div>
        ))}
      </div>
    </AppContext.Provider>
  );
}

export function useApp() {
  const context = useContext(AppContext);
  if (context === undefined) {
    throw new Error('useApp must be used within an AppProvider');
  }
  return context;
}
