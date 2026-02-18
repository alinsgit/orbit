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

// Site types
export type WebServer = 'nginx' | 'apache';

export interface Site {
  domain: string;
  path: string;
  port: number;
  php_version?: string;
  php_port?: number;
  ssl_enabled?: boolean;
  template?: 'http' | 'laravel' | 'wordpress' | 'litecart' | 'static';
  web_server?: WebServer;
}

export interface SiteWithStatus extends Site {
  created_at?: string;
  config_valid: boolean;
  warning?: string;
  web_server: WebServer;
}

// Site management
export const createSite = async (site: Site): Promise<SiteWithStatus> => {
  try {
    return await invoke('create_site', { site });
  } catch (error) {
    console.error('Failed to create site:', error);
    throw error;
  }
};

export const getSites = async (): Promise<SiteWithStatus[]> => {
  try {
    return await invoke('get_sites');
  } catch (error) {
    console.error('Failed to fetch sites:', error);
    throw error;
  }
};

export const getSite = async (domain: string): Promise<SiteWithStatus | null> => {
  try {
    return await invoke('get_site', { domain });
  } catch (error) {
    console.error('Failed to fetch site:', error);
    throw error;
  }
};

export const updateSite = async (domain: string, site: Site): Promise<SiteWithStatus> => {
  try {
    return await invoke('update_site', { domain, site });
  } catch (error) {
    console.error('Failed to update site:', error);
    throw error;
  }
};

export const deleteSite = async (domain: string): Promise<string> => {
  try {
    return await invoke('delete_site', { domain });
  } catch (error) {
    console.error('Failed to delete site:', error);
    throw error;
  }
};

export const regenerateSiteConfig = async (domain: string): Promise<string> => {
  try {
    return await invoke('regenerate_site_config', { domain });
  } catch (error) {
    console.error('Failed to regenerate config:', error);
    throw error;
  }
};

// Export/Import Sites
export interface SiteExport {
  version: string;
  exported_at: string;
  sites: SiteExportEntry[];
}

export interface SiteExportEntry {
  name: string;
  domain: string;
  root_path: string;
  php_version?: string;
  ssl_enabled: boolean;
  template?: string;
  web_server: string;
}

export interface ImportResult {
  imported: number;
  skipped: number;
  errors: string[];
}

export const exportSites = async (): Promise<SiteExport> => {
  try {
    return await invoke('export_sites');
  } catch (error) {
    console.error('Failed to export sites:', error);
    throw error;
  }
};

export const importSites = async (importData: SiteExport, skipExisting: boolean = true): Promise<ImportResult> => {
  try {
    return await invoke('import_sites', { importData, skipExisting });
  } catch (error) {
    console.error('Failed to import sites:', error);
    throw error;
  }
};

// Nginx management
export const nginxTestConfig = async (): Promise<string> => {
  try {
    return await invoke('nginx_test_config');
  } catch (error) {
    console.error('Failed to test nginx config:', error);
    throw error;
  }
};

export const nginxReload = async (): Promise<string> => {
  try {
    return await invoke('nginx_reload');
  } catch (error) {
    console.error('Failed to reload nginx:', error);
    throw error;
  }
};

export const nginxStatus = async (): Promise<boolean> => {
  try {
    return await invoke('nginx_status');
  } catch (error) {
    console.error('Failed to get nginx status:', error);
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

// Hosts file management
export const addHostElevated = async (domain: string): Promise<string> => {
  try {
    return await invoke('add_host_elevated', { domain });
  } catch (error) {
    console.error('Failed to add host:', error);
    throw error;
  }
};

// PATH management
export interface PathStatus {
  in_path: boolean;
  bin_dir: string;
}

export interface ServicePathStatus {
  in_path: boolean;
  service_path: string;
  service_type: string;
}

export const addToPath = async (): Promise<string> => {
  try {
    return await invoke('add_to_path');
  } catch (error) {
    console.error('Failed to add to PATH:', error);
    throw error;
  }
};

export const checkPathStatus = async (): Promise<PathStatus> => {
  try {
    return await invoke('check_path_status');
  } catch (error) {
    console.error('Failed to check PATH status:', error);
    throw error;
  }
};

export const removeFromPath = async (): Promise<string> => {
  try {
    return await invoke('remove_from_path');
  } catch (error) {
    console.error('Failed to remove from PATH:', error);
    throw error;
  }
};

// Per-service PATH management
export const addServiceToPath = async (serviceType: string): Promise<string> => {
  try {
    return await invoke('add_service_to_path', { serviceType });
  } catch (error) {
    console.error('Failed to add service to PATH:', error);
    throw error;
  }
};

export const removeServiceFromPath = async (serviceType: string): Promise<string> => {
  try {
    return await invoke('remove_service_from_path', { serviceType });
  } catch (error) {
    console.error('Failed to remove service from PATH:', error);
    throw error;
  }
};

export const checkServicePathStatus = async (serviceType: string): Promise<ServicePathStatus> => {
  try {
    return await invoke('check_service_path_status', { serviceType });
  } catch (error) {
    console.error('Failed to check service PATH status:', error);
    throw error;
  }
};

// PHP Config types
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

// PHP Config API
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

// Log Management Types
export interface LogFile {
  name: string;
  path: string;
  size: number;
  modified: string;
  log_type: string;
}

export interface LogEntry {
  timestamp: string | null;
  level: string;
  message: string;
  raw: string;
}

// Log Management API
export const getLogFiles = async (): Promise<LogFile[]> => {
  try {
    return await invoke('get_log_files');
  } catch (error) {
    console.error('Failed to get log files:', error);
    throw error;
  }
};

export const readLogFile = async (path: string, lines: number, offset: number): Promise<LogEntry[]> => {
  try {
    return await invoke('read_log_file', { path, lines, offset });
  } catch (error) {
    console.error('Failed to read log file:', error);
    throw error;
  }
};

export const clearLogFile = async (path: string): Promise<void> => {
  try {
    return await invoke('clear_log_file', { path });
  } catch (error) {
    console.error('Failed to clear log file:', error);
    throw error;
  }
};

export const clearAllLogs = async (): Promise<number> => {
  try {
    return await invoke('clear_all_logs');
  } catch (error) {
    console.error('Failed to clear all logs:', error);
    throw error;
  }
};

// SSL Types
export interface SslCertificate {
  domain: string;
  cert_path: string;
  key_path: string;
  created_at: string;
  is_valid: boolean;
}

export interface SslStatus {
  mkcert_installed: boolean;
  mkcert_path: string;
  ca_installed: boolean;
  certificates: SslCertificate[];
}

// SSL API
export const getSslStatus = async (): Promise<SslStatus> => {
  try {
    return await invoke('get_ssl_status');
  } catch (error) {
    console.error('Failed to get SSL status:', error);
    throw error;
  }
};

export const installMkcert = async (): Promise<string> => {
  try {
    return await invoke('install_mkcert');
  } catch (error) {
    console.error('Failed to install mkcert:', error);
    throw error;
  }
};

export const installSslCa = async (): Promise<string> => {
  try {
    return await invoke('install_ssl_ca');
  } catch (error) {
    console.error('Failed to install SSL CA:', error);
    throw error;
  }
};

export const generateSslCert = async (domain: string): Promise<SslCertificate> => {
  try {
    return await invoke('generate_ssl_cert', { domain });
  } catch (error) {
    console.error('Failed to generate SSL cert:', error);
    throw error;
  }
};

export const getSslCert = async (domain: string): Promise<SslCertificate | null> => {
  try {
    return await invoke('get_ssl_cert', { domain });
  } catch (error) {
    console.error('Failed to get SSL cert:', error);
    throw error;
  }
};

export const listSslCerts = async (): Promise<SslCertificate[]> => {
  try {
    return await invoke('list_ssl_certs');
  } catch (error) {
    console.error('Failed to list SSL certs:', error);
    throw error;
  }
};

export const deleteSslCert = async (domain: string): Promise<void> => {
  try {
    return await invoke('delete_ssl_cert', { domain });
  } catch (error) {
    console.error('Failed to delete SSL cert:', error);
    throw error;
  }
};

// Database Types
export interface DatabaseStatus {
  adminer_installed: boolean;
  adminer_path: string;
  adminer_url: string;
}

export interface PhpMyAdminStatus {
  installed: boolean;
  path: string;
  url: string;
  version: string;
}

export interface DatabaseToolsStatus {
  adminer: DatabaseStatus;
  phpmyadmin: PhpMyAdminStatus;
}

// Database API
export const getDatabaseStatus = async (): Promise<DatabaseStatus> => {
  try {
    return await invoke('get_database_status');
  } catch (error) {
    console.error('Failed to get database status:', error);
    throw error;
  }
};

export const getDatabaseToolsStatus = async (): Promise<DatabaseToolsStatus> => {
  try {
    return await invoke('get_database_tools_status');
  } catch (error) {
    console.error('Failed to get database tools status:', error);
    throw error;
  }
};

export const installAdminer = async (): Promise<string> => {
  try {
    return await invoke('install_adminer');
  } catch (error) {
    console.error('Failed to install Adminer:', error);
    throw error;
  }
};

export const uninstallAdminer = async (): Promise<void> => {
  try {
    return await invoke('uninstall_adminer');
  } catch (error) {
    console.error('Failed to uninstall Adminer:', error);
    throw error;
  }
};

export const setupAdminerNginx = async (phpPort: number): Promise<string> => {
  try {
    return await invoke('setup_adminer_nginx', { phpPort });
  } catch (error) {
    console.error('Failed to setup Adminer nginx:', error);
    throw error;
  }
};

export const removeAdminerNginx = async (): Promise<void> => {
  try {
    return await invoke('remove_adminer_nginx');
  } catch (error) {
    console.error('Failed to remove Adminer nginx:', error);
    throw error;
  }
};

// PhpMyAdmin API
export const getPhpMyAdminStatus = async (): Promise<PhpMyAdminStatus> => {
  try {
    return await invoke('get_phpmyadmin_status');
  } catch (error) {
    console.error('Failed to get PhpMyAdmin status:', error);
    throw error;
  }
};

export const installPhpMyAdmin = async (): Promise<string> => {
  try {
    return await invoke('install_phpmyadmin');
  } catch (error) {
    console.error('Failed to install PhpMyAdmin:', error);
    throw error;
  }
};

export const uninstallPhpMyAdmin = async (): Promise<void> => {
  try {
    return await invoke('uninstall_phpmyadmin');
  } catch (error) {
    console.error('Failed to uninstall PhpMyAdmin:', error);
    throw error;
  }
};

export const setupPhpMyAdminNginx = async (phpPort: number): Promise<string> => {
  try {
    return await invoke('setup_phpmyadmin_nginx', { phpPort });
  } catch (error) {
    console.error('Failed to setup PhpMyAdmin nginx:', error);
    throw error;
  }
};

export const removePhpMyAdminNginx = async (): Promise<void> => {
  try {
    return await invoke('remove_phpmyadmin_nginx');
  } catch (error) {
    console.error('Failed to remove PhpMyAdmin nginx:', error);
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
    // Return default safe values if check fails
    return { vc_redist_installed: true };
  }
};

// Template Types
export interface TemplateInfo {
  name: string;
  description: string;
  is_custom: boolean;
  path?: string;
}

// Template API
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

// PHP Registry Types
export interface PhpService {
  version: string;
  port: number;
  path: string;
  status: 'running' | 'stopped';
  pid?: number;
}

// PHP Registry API
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

// Cache (Redis) Types
export interface CacheStatus {
  redis_installed: boolean;
  redis_path: string | null;
  redis_running: boolean;
  redis_port: number;
}

// Cache API
export const getCacheStatus = async (): Promise<CacheStatus> => {
  try {
    return await invoke('get_cache_status');
  } catch (error) {
    console.error('Failed to get cache status:', error);
    throw error;
  }
};

export const installRedis = async (): Promise<string> => {
  try {
    return await invoke('install_redis');
  } catch (error) {
    console.error('Failed to install Redis:', error);
    throw error;
  }
};

export const uninstallRedis = async (): Promise<string> => {
  try {
    return await invoke('uninstall_redis');
  } catch (error) {
    console.error('Failed to uninstall Redis:', error);
    throw error;
  }
};

export const updateRedisConfig = async (port: number, maxMemory: string): Promise<string> => {
  try {
    return await invoke('update_redis_config', { port, maxMemory });
  } catch (error) {
    console.error('Failed to update Redis config:', error);
    throw error;
  }
};

export const getRedisExePath = async (): Promise<string> => {
  try {
    return await invoke('get_redis_exe_path');
  } catch (error) {
    console.error('Failed to get Redis exe path:', error);
    throw error;
  }
};

// Composer Types
export interface ComposerStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
  php_version: string | null;
}

export interface ComposerDependency {
  name: string;
  version: string;
  installed_version: string | null;
}

export interface ComposerProject {
  name: string | null;
  description: string | null;
  dependencies: ComposerDependency[];
  dev_dependencies: ComposerDependency[];
}

// Composer API
export const getComposerStatus = async (): Promise<ComposerStatus> => {
  try {
    return await invoke('get_composer_status');
  } catch (error) {
    console.error('Failed to get Composer status:', error);
    throw error;
  }
};

export const installComposer = async (): Promise<string> => {
  try {
    return await invoke('install_composer');
  } catch (error) {
    console.error('Failed to install Composer:', error);
    throw error;
  }
};

export const uninstallComposer = async (): Promise<string> => {
  try {
    return await invoke('uninstall_composer');
  } catch (error) {
    console.error('Failed to uninstall Composer:', error);
    throw error;
  }
};

export const updateComposer = async (): Promise<string> => {
  try {
    return await invoke('update_composer');
  } catch (error) {
    console.error('Failed to update Composer:', error);
    throw error;
  }
};

export const composerInstall = async (projectPath: string): Promise<string> => {
  try {
    return await invoke('composer_install', { projectPath });
  } catch (error) {
    console.error('Failed to run composer install:', error);
    throw error;
  }
};

export const composerUpdate = async (projectPath: string): Promise<string> => {
  try {
    return await invoke('composer_update', { projectPath });
  } catch (error) {
    console.error('Failed to run composer update:', error);
    throw error;
  }
};

export const composerRequire = async (projectPath: string, packageName: string, dev: boolean = false): Promise<string> => {
  try {
    return await invoke('composer_require', { projectPath, package: packageName, dev });
  } catch (error) {
    console.error('Failed to require package:', error);
    throw error;
  }
};

export const composerRemove = async (projectPath: string, packageName: string): Promise<string> => {
  try {
    return await invoke('composer_remove', { projectPath, package: packageName });
  } catch (error) {
    console.error('Failed to remove package:', error);
    throw error;
  }
};

export const getComposerProject = async (projectPath: string): Promise<ComposerProject> => {
  try {
    return await invoke('get_composer_project', { projectPath });
  } catch (error) {
    console.error('Failed to get project info:', error);
    throw error;
  }
};

export const composerRun = async (projectPath: string, args: string[]): Promise<string> => {
  try {
    return await invoke('composer_run', { projectPath, args });
  } catch (error) {
    console.error('Failed to run composer command:', error);
    throw error;
  }
};

// Performance Types
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

// Performance API
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

// Cache Clear Types
export interface CacheClearResult {
  opcache_cleared: boolean;
  temp_files_cleared: number;
  nginx_cache_cleared: boolean;
  message: string;
}

// Cache Clear API
export const clearAllCaches = async (phpVersion?: string): Promise<CacheClearResult> => {
  try {
    return await invoke('clear_all_caches', { phpVersion });
  } catch (error) {
    console.error('Failed to clear caches:', error);
    throw error;
  }
};

// Mailpit (Mail Server) Types
export interface MailpitStatus {
  installed: boolean;
  running: boolean;
  path: string | null;
  smtp_port: number;
  web_port: number;
}

// Mailpit API
export const getMailpitStatus = async (): Promise<MailpitStatus> => {
  try {
    return await invoke('get_mailpit_status');
  } catch (error) {
    console.error('Failed to get Mailpit status:', error);
    throw error;
  }
};

export const installMailpit = async (): Promise<string> => {
  try {
    return await invoke('install_mailpit');
  } catch (error) {
    console.error('Failed to install Mailpit:', error);
    throw error;
  }
};

export const uninstallMailpit = async (): Promise<string> => {
  try {
    return await invoke('uninstall_mailpit');
  } catch (error) {
    console.error('Failed to uninstall Mailpit:', error);
    throw error;
  }
};

export const startMailpit = async (): Promise<string> => {
  try {
    return await invoke('start_mailpit');
  } catch (error) {
    console.error('Failed to start Mailpit:', error);
    throw error;
  }
};

export const stopMailpit = async (): Promise<string> => {
  try {
    return await invoke('stop_mailpit');
  } catch (error) {
    console.error('Failed to stop Mailpit:', error);
    throw error;
  }
};

export const getMailpitExePath = async (): Promise<string> => {
  try {
    return await invoke('get_mailpit_exe_path');
  } catch (error) {
    console.error('Failed to get Mailpit exe path:', error);
    throw error;
  }
};

// ============================================================================
// PECL Extension Manager
// ============================================================================

export interface PeclExtension {
  name: string;
  version: string;
  description: string;
  download_url: string | null;
  installed: boolean;
  enabled: boolean;
  category: string;
}

/**
 * Get list of available PECL extensions for a PHP version
 */
export const getAvailableExtensions = async (phpVersion: string): Promise<PeclExtension[]> => {
  try {
    return await invoke('get_available_extensions', { phpVersion });
  } catch (error) {
    console.error('Failed to get available extensions:', error);
    throw error;
  }
};

/**
 * Install a PECL extension
 */
export const installPeclExtension = async (phpVersion: string, extensionName: string): Promise<string> => {
  try {
    return await invoke('install_pecl_extension', { phpVersion, extensionName });
  } catch (error) {
    console.error('Failed to install PECL extension:', error);
    throw error;
  }
};

/**
 * Uninstall a PECL extension
 */
export const uninstallPeclExtension = async (phpVersion: string, extensionName: string): Promise<string> => {
  try {
    return await invoke('uninstall_pecl_extension', { phpVersion, extensionName });
  } catch (error) {
    console.error('Failed to uninstall PECL extension:', error);
    throw error;
  }
};

/**
 * Search for PECL extensions
 */
export const searchPeclExtensions = async (query: string, phpVersion: string): Promise<PeclExtension[]> => {
  try {
    return await invoke('search_pecl_extensions', { query, phpVersion });
  } catch (error) {
    console.error('Failed to search PECL extensions:', error);
    throw error;
  }
};

// ==========================================
// Database Backup/Import API
// ==========================================

export const exportDatabase = async (database: string, outputPath: string): Promise<string> => {
  try {
    return await invoke('export_database', { database, outputPath });
  } catch (error) {
    console.error('Failed to export database:', error);
    throw error;
  }
};

export const exportAllDatabases = async (outputPath: string): Promise<string> => {
  try {
    return await invoke('export_all_databases', { outputPath });
  } catch (error) {
    console.error('Failed to export all databases:', error);
    throw error;
  }
};

export const importSql = async (database: string, sqlPath: string): Promise<string> => {
  try {
    return await invoke('import_sql', { database, sqlPath });
  } catch (error) {
    console.error('Failed to import SQL:', error);
    throw error;
  }
};

export const rebuildDatabase = async (database: string, sqlPath: string): Promise<string> => {
  try {
    return await invoke('rebuild_database', { database, sqlPath });
  } catch (error) {
    console.error('Failed to rebuild database:', error);
    throw error;
  }
};
