import { invoke } from '@tauri-apps/api/core';

// PHP Config
export interface PhpExtension {
  name: string;
  enabled: boolean;
  available: boolean;
}

export interface PhpConfig {
  version: string;
  path: string;
  extensions: PhpExtension[];
  settings: Record<string, string>;
}

export const getPhpConfig = async (version: string): Promise<PhpConfig> => {
  try {
    return await invoke('get_php_config', { version });
  } catch (error) {
    console.error('Failed to get PHP config:', error);
    throw error;
  }
};

export const setPhpExtension = async (version: string, extension: string, enabled: boolean): Promise<string> => {
  try {
    return await invoke('set_php_extension', { version, extension, enabled });
  } catch (error) {
    console.error('Failed to set PHP extension:', error);
    throw error;
  }
};

export const setPhpSetting = async (version: string, key: string, value: string): Promise<string> => {
  try {
    return await invoke('set_php_setting', { version, key, value });
  } catch (error) {
    console.error('Failed to set PHP setting:', error);
    throw error;
  }
};

export const getPhpIniRaw = async (version: string): Promise<string> => {
  try {
    return await invoke('get_php_ini_raw', { version });
  } catch (error) {
    console.error('Failed to get php.ini:', error);
    throw error;
  }
};

export const savePhpIniRaw = async (version: string, content: string): Promise<string> => {
  try {
    return await invoke('save_php_ini_raw', { version, content });
  } catch (error) {
    console.error('Failed to save php.ini:', error);
    throw error;
  }
};

// Nginx Config
export const getNginxConfRaw = async (): Promise<string> => {
  try {
    return await invoke('get_nginx_conf_raw');
  } catch (error) {
    console.error('Failed to get nginx.conf:', error);
    throw error;
  }
};

export const saveNginxConfRaw = async (content: string): Promise<string> => {
  try {
    return await invoke('save_nginx_conf_raw', { content });
  } catch (error) {
    console.error('Failed to save nginx.conf:', error);
    throw error;
  }
};

// MariaDB Config
export const getMariadbConfRaw = async (): Promise<string> => {
  try {
    return await invoke('get_mariadb_conf_raw');
  } catch (error) {
    console.error('Failed to get my.ini:', error);
    throw error;
  }
};

export const saveMariadbConfRaw = async (content: string): Promise<string> => {
  try {
    return await invoke('save_mariadb_conf_raw', { content });
  } catch (error) {
    console.error('Failed to save my.ini:', error);
    throw error;
  }
};

// Apache Config
export const getApacheConfRaw = async (): Promise<string> => {
  try {
    return await invoke('get_apache_conf_raw');
  } catch (error) {
    console.error('Failed to get httpd.conf:', error);
    throw error;
  }
};

export const saveApacheConfRaw = async (content: string): Promise<string> => {
  try {
    return await invoke('save_apache_conf_raw', { content });
  } catch (error) {
    console.error('Failed to save httpd.conf:', error);
    throw error;
  }
};

// PHP Mailpit Integration
export const configurePhpMailpit = async (version: string, enabled: boolean, smtpPort: number = 1025): Promise<string> => {
  try {
    return await invoke('configure_php_mailpit', { version, enabled, smtpPort });
  } catch (error) {
    console.error('Failed to configure PHP Mailpit:', error);
    throw error;
  }
};

export const getPhpMailpitStatus = async (version: string): Promise<boolean> => {
  try {
    return await invoke('get_php_mailpit_status', { version });
  } catch (error) {
    console.error('Failed to get PHP Mailpit status:', error);
    return false;
  }
};

// PHP Redis Session Integration
export const configurePhpRedisSession = async (version: string, enabled: boolean, redisPort: number = 6379): Promise<string> => {
  try {
    return await invoke('configure_php_redis_session', { version, enabled, redisPort });
  } catch (error) {
    console.error('Failed to configure PHP Redis session:', error);
    throw error;
  }
};

export const getPhpRedisSessionStatus = async (version: string): Promise<boolean> => {
  try {
    return await invoke('get_php_redis_session_status', { version });
  } catch (error) {
    console.error('Failed to get PHP Redis session status:', error);
    return false;
  }
};

// Performance
export interface PerformanceStatus {
  opcache_enabled: boolean;
  opcache_memory: string;
  nginx_gzip_enabled: boolean;
  nginx_gzip_level: number;
}

export interface OpcacheConfig {
  enabled: boolean;
  memory: string;
  max_files: string;
  validate_timestamps: boolean;
  revalidate_freq: string;
}

export interface NginxGzipConfig {
  enabled: boolean;
  level: number;
  min_length: string;
  types: string[];
}

export const getPerformanceStatus = async (phpVersion?: string): Promise<PerformanceStatus> => {
  try {
    return await invoke('get_performance_status', { phpVersion });
  } catch (error) {
    console.error('Failed to get performance status:', error);
    throw error;
  }
};

export const getOpcacheConfig = async (version: string): Promise<OpcacheConfig> => {
  try {
    return await invoke('get_opcache_config', { version });
  } catch (error) {
    console.error('Failed to get OPcache config:', error);
    throw error;
  }
};

export const setOpcacheConfig = async (version: string, config: OpcacheConfig): Promise<string> => {
  try {
    return await invoke('set_opcache_config', { version, config });
  } catch (error) {
    console.error('Failed to set OPcache config:', error);
    throw error;
  }
};

export const getNginxGzipConfig = async (): Promise<NginxGzipConfig> => {
  try {
    return await invoke('get_nginx_gzip_config');
  } catch (error) {
    console.error('Failed to get Nginx gzip config:', error);
    throw error;
  }
};

export const setNginxGzipConfig = async (config: NginxGzipConfig): Promise<string> => {
  try {
    return await invoke('set_nginx_gzip_config', { config });
  } catch (error) {
    console.error('Failed to set Nginx gzip config:', error);
    throw error;
  }
};

// Cache Clear
export interface CacheClearResult {
  opcache_cleared: boolean;
  temp_files_cleared: number;
  nginx_cache_cleared: boolean;
  message: string;
}

export const clearAllCaches = async (phpVersion?: string): Promise<CacheClearResult> => {
  try {
    return await invoke('clear_all_caches', { phpVersion });
  } catch (error) {
    console.error('Failed to clear caches:', error);
    throw error;
  }
};

// Templates
export interface TemplateInfo {
  name: string;
  description: string;
  is_custom: boolean;
  path?: string;
}

export const listTemplates = async (): Promise<TemplateInfo[]> => {
  try {
    return await invoke('list_templates');
  } catch (error) {
    console.error('Failed to list templates:', error);
    throw error;
  }
};

export const getTemplate = async (name: string): Promise<string> => {
  try {
    return await invoke('get_template', { name });
  } catch (error) {
    console.error('Failed to get template:', error);
    throw error;
  }
};

export const saveTemplate = async (name: string, content: string): Promise<void> => {
  try {
    return await invoke('save_template', { name, content });
  } catch (error) {
    console.error('Failed to save template:', error);
    throw error;
  }
};

export const resetTemplate = async (name: string): Promise<void> => {
  try {
    return await invoke('reset_template', { name });
  } catch (error) {
    console.error('Failed to reset template:', error);
    throw error;
  }
};

export const deleteTemplate = async (name: string): Promise<void> => {
  try {
    return await invoke('delete_template', { name });
  } catch (error) {
    console.error('Failed to delete template:', error);
    throw error;
  }
};

// PHP Registry
export interface PhpService {
  version: string;
  port: number;
  path: string;
  status: 'running' | 'stopped';
  pid?: number;
}

export const getPhpServices = async (): Promise<PhpService[]> => {
  try {
    return await invoke('get_php_services');
  } catch (error) {
    console.error('Failed to get PHP services:', error);
    throw error;
  }
};

export const getPhpService = async (version: string): Promise<PhpService | null> => {
  try {
    return await invoke('get_php_service', { version });
  } catch (error) {
    console.error('Failed to get PHP service:', error);
    throw error;
  }
};

export const getPhpPort = async (version: string): Promise<number> => {
  try {
    return await invoke('get_php_port', { version });
  } catch (error) {
    console.error('Failed to get PHP port:', error);
    throw error;
  }
};

export const registerPhpVersion = async (version: string, path: string): Promise<PhpService> => {
  try {
    return await invoke('register_php_version', { version, path });
  } catch (error) {
    console.error('Failed to register PHP version:', error);
    throw error;
  }
};

export const unregisterPhpVersion = async (version: string): Promise<boolean> => {
  try {
    return await invoke('unregister_php_version', { version });
  } catch (error) {
    console.error('Failed to unregister PHP version:', error);
    throw error;
  }
};

export const markPhpRunning = async (version: string, pid: number): Promise<boolean> => {
  try {
    return await invoke('mark_php_running', { version, pid });
  } catch (error) {
    console.error('Failed to mark PHP running:', error);
    throw error;
  }
};

export const markPhpStopped = async (version: string): Promise<boolean> => {
  try {
    return await invoke('mark_php_stopped', { version });
  } catch (error) {
    console.error('Failed to mark PHP stopped:', error);
    throw error;
  }
};

export const scanPhpVersions = async (): Promise<number> => {
  try {
    return await invoke('scan_php_versions');
  } catch (error) {
    console.error('Failed to scan PHP versions:', error);
    throw error;
  }
};

export const getRunningPhpServices = async (): Promise<PhpService[]> => {
  try {
    return await invoke('get_running_php_services');
  } catch (error) {
    console.error('Failed to get running PHP services:', error);
    throw error;
  }
};

export const calculatePhpPort = async (version: string): Promise<number> => {
  try {
    return await invoke('calculate_php_port', { version });
  } catch (error) {
    console.error('Failed to calculate PHP port:', error);
    throw error;
  }
};

// PECL Extension Manager
export interface PeclExtension {
  name: string;
  version: string;
  description: string;
  download_url: string | null;
  installed: boolean;
  enabled: boolean;
  category: string;
}

export const getAvailableExtensions = async (phpVersion: string): Promise<PeclExtension[]> => {
  try {
    return await invoke('get_available_extensions', { phpVersion });
  } catch (error) {
    console.error('Failed to get available extensions:', error);
    throw error;
  }
};

export const installPeclExtension = async (phpVersion: string, extensionName: string): Promise<string> => {
  try {
    return await invoke('install_pecl_extension', { phpVersion, extensionName });
  } catch (error) {
    console.error('Failed to install PECL extension:', error);
    throw error;
  }
};

export const uninstallPeclExtension = async (phpVersion: string, extensionName: string): Promise<string> => {
  try {
    return await invoke('uninstall_pecl_extension', { phpVersion, extensionName });
  } catch (error) {
    console.error('Failed to uninstall PECL extension:', error);
    throw error;
  }
};

export const searchPeclExtensions = async (query: string, phpVersion: string): Promise<PeclExtension[]> => {
  try {
    return await invoke('search_pecl_extensions', { query, phpVersion });
  } catch (error) {
    console.error('Failed to search PECL extensions:', error);
    throw error;
  }
};
