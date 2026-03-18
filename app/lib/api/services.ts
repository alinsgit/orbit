import { invoke } from '@tauri-apps/api/core';

export interface ServiceStatus {
  name: string;
  status: 'running' | 'stopped';
  pid?: number;
}

export interface ServiceVersion {
  version: string;
  download_url: string;
  filename: string;
  release_date?: string;
  source?: 'Api' | 'Cache' | 'Fallback';
}

export interface InstalledService {
  name: string;
  version: string;
  path: string;
  service_type: string;
  port?: number;
}

export const startService = async (name: string, binPath: string): Promise<string> => {
  try {
    return await invoke('start_service', { name, binPath });
  } catch (error) {
    console.error('Failed to start service:', error);
    throw error;
  }
};

export const stopService = async (name: string): Promise<string> => {
  try {
    return await invoke('stop_service', { name });
  } catch (error) {
    console.error('Failed to stop service:', error);
    throw error;
  }
};

export const reloadService = async (name: string): Promise<string> => {
  try {
    return await invoke('reload_service', { name });
  } catch (error) {
    console.error('Failed to reload service:', error);
    throw error;
  }
};

export const downloadService = async (url: string, filename: string, serviceType: string): Promise<string> => {
  try {
    return await invoke('download_service', { url, filename, serviceType });
  } catch (error) {
    console.error('Failed to download service:', error);
    throw error;
  }
};

export const getAvailableVersions = async (service: string, forceRefresh?: boolean): Promise<ServiceVersion[]> => {
  try {
    return await invoke('get_available_versions', { service, forceRefresh });
  } catch (error) {
    console.error('Failed to fetch versions:', error);
    throw error;
  }
};

export const refreshAllVersions = async (): Promise<void> => {
  try {
    return await invoke('refresh_all_versions');
  } catch (error) {
    console.error('Failed to refresh versions:', error);
    throw error;
  }
};

export const getInstalledServices = async (): Promise<InstalledService[]> => {
  try {
    return await invoke('get_installed_services');
  } catch (error) {
    console.error('Failed to fetch installed services:', error);
    throw error;
  }
};

export const getServiceStatus = async (name: string): Promise<string> => {
  try {
    return await invoke('get_service_status', { name });
  } catch (error) {
    console.error('Failed to get service status:', error);
    throw error;
  }
};

export const assignPhpPort = async (phpVersion: number, startPort: number): Promise<number> => {
  try {
    return await invoke('assign_php_port', { phpVersion, startPort });
  } catch (error) {
    console.error('Failed to assign PHP port:', error);
    throw error;
  }
};

export const checkPortConflict = async (port: number): Promise<boolean> => {
  try {
    return await invoke('check_port_conflict', { port });
  } catch (error) {
    console.error('Failed to check port conflict:', error);
    throw error;
  }
};

export const initializeMariaDB = async (rootPassword: string): Promise<string> => {
  try {
    return await invoke('initialize_mariadb', { rootPassword });
  } catch (error) {
    console.error('Failed to initialize MariaDB:', error);
    throw error;
  }
};

export const uninstallService = async (name: string, serviceType: string, path: string): Promise<string> => {
  try {
    return await invoke('uninstall_service', { name, serviceType, path });
  } catch (error) {
    console.error('Failed to uninstall service:', error);
    throw error;
  }
};

export const autoStartServices = async (installedServices: string[]): Promise<string[]> => {
  try {
    return await invoke('auto_start_services', { installedServices });
  } catch (error) {
    console.error('Failed to auto-start services:', error);
    throw error;
  }
};

// System Requirements
export interface SystemRequirements {
  vc_redist_installed: boolean;
}

export const checkSystemRequirements = async (): Promise<SystemRequirements> => {
  try {
    return await invoke('check_system_requirements');
  } catch (error) {
    console.error('Failed to check system requirements:', error);
    return { vc_redist_installed: true };
  }
};
