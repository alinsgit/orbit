import { invoke } from '@tauri-apps/api/core';

// Hosts
export const addHostElevated = async (domain: string): Promise<string> => {
  try {
    return await invoke('add_host_elevated', { domain });
  } catch (error) {
    console.error('Failed to add host:', error);
    throw error;
  }
};

export const getHostsFile = async (): Promise<string> => {
  try {
    return await invoke('get_hosts_file');
  } catch (error) {
    console.error('Failed to get hosts file:', error);
    throw error;
  }
};

export const saveHostsFile = async (newContent: string): Promise<string> => {
  try {
    return await invoke('save_hosts_file', { newContent });
  } catch (error) {
    console.error('Failed to save hosts file:', error);
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

export const getUserPath = async (): Promise<string[]> => {
  try {
    return await invoke('get_user_path');
  } catch (error) {
    console.error('Failed to get user path:', error);
    throw error;
  }
};

export const saveUserPath = async (paths: string[]): Promise<string> => {
  try {
    return await invoke('save_user_path', { paths });
  } catch (error) {
    console.error('Failed to save user path:', error);
    throw error;
  }
};

// SSL
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

// Logs
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

export interface LogReadResult {
  entries: LogEntry[];
  total_lines: number;
  filtered_lines: number;
}

export const getLogFiles = async (): Promise<LogFile[]> => {
  try {
    return await invoke('get_log_files');
  } catch (error) {
    console.error('Failed to get log files:', error);
    throw error;
  }
};

export const readLogFile = async (
  path: string,
  lines: number,
  offset: number,
  levelFilter?: string,
  searchQuery?: string,
): Promise<LogReadResult> => {
  try {
    return await invoke('read_log_file', {
      path, lines, offset,
      levelFilter: levelFilter || null,
      searchQuery: searchQuery || null,
    });
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

// Cache (Redis)
export interface CacheStatus {
  redis_installed: boolean;
  redis_path: string | null;
  redis_running: boolean;
  redis_port: number;
}

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

// Composer
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

// Mailpit
export interface MailpitStatus {
  installed: boolean;
  running: boolean;
  path: string | null;
  smtp_port: number;
  web_port: number;
}

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

// Meilisearch
export interface MeilisearchStatus {
  installed: boolean;
  running: boolean;
  path: string | null;
  http_port: number;
}

export const getMeilisearchStatus = async (): Promise<MeilisearchStatus> => {
  try {
    return await invoke('get_meilisearch_status');
  } catch (error) {
    console.error('Failed to get Meilisearch status:', error);
    throw error;
  }
};

export const installMeilisearch = async (): Promise<string> => {
  try {
    return await invoke('install_meilisearch');
  } catch (error) {
    console.error('Failed to install Meilisearch:', error);
    throw error;
  }
};

export const uninstallMeilisearch = async (): Promise<string> => {
  try {
    return await invoke('uninstall_meilisearch');
  } catch (error) {
    console.error('Failed to uninstall Meilisearch:', error);
    throw error;
  }
};

export const startMeilisearch = async (): Promise<string> => {
  try {
    return await invoke('start_meilisearch');
  } catch (error) {
    console.error('Failed to start Meilisearch:', error);
    throw error;
  }
};

export const stopMeilisearch = async (): Promise<string> => {
  try {
    return await invoke('stop_meilisearch');
  } catch (error) {
    console.error('Failed to stop Meilisearch:', error);
    throw error;
  }
};

export const getMeilisearchExePath = async (): Promise<string> => {
  try {
    return await invoke('get_meilisearch_exe_path');
  } catch (error) {
    console.error('Failed to get Meilisearch exe path:', error);
    throw error;
  }
};

// MCP Server
export interface McpStatus {
  installed: boolean;
  running: boolean;
  path: string | null;
  pid: number | null;
  binary_exists: boolean;
}

export interface BinaryUpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
}

export const getMcpStatus = async (): Promise<McpStatus> => {
  try {
    return await invoke('get_mcp_status');
  } catch (error) {
    console.error('Failed to get MCP status:', error);
    throw error;
  }
};

export const installMcp = async (): Promise<string> => {
  try {
    return await invoke('install_mcp');
  } catch (error) {
    console.error('Failed to install MCP:', error);
    throw error;
  }
};

export const uninstallMcp = async (): Promise<string> => {
  try {
    return await invoke('uninstall_mcp');
  } catch (error) {
    console.error('Failed to uninstall MCP:', error);
    throw error;
  }
};

export const startMcp = async (): Promise<string> => {
  try {
    return await invoke('start_mcp');
  } catch (error) {
    console.error('Failed to start MCP:', error);
    throw error;
  }
};

export const stopMcp = async (): Promise<string> => {
  try {
    return await invoke('stop_mcp');
  } catch (error) {
    console.error('Failed to stop MCP:', error);
    throw error;
  }
};

export const getMcpBinaryPath = async (): Promise<string> => {
  try {
    return await invoke('get_mcp_binary_path');
  } catch (error) {
    console.error('Failed to get MCP binary path:', error);
    throw error;
  }
};

export const checkMcpUpdate = async (): Promise<BinaryUpdateInfo> => {
  try {
    return await invoke('check_mcp_update');
  } catch (error) {
    console.error('Failed to check MCP update:', error);
    throw error;
  }
};

export const updateMcp = async (): Promise<string> => {
  try {
    return await invoke('update_mcp');
  } catch (error) {
    console.error('Failed to update MCP:', error);
    throw error;
  }
};

// CLI
export interface CliStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
  binary_exists: boolean;
}

export const getCliStatus = async (): Promise<CliStatus> => {
  try {
    return await invoke('get_cli_status');
  } catch (error) {
    console.error('Failed to get CLI status:', error);
    throw error;
  }
};

export const installCli = async (): Promise<string> => {
  try {
    return await invoke('install_cli');
  } catch (error) {
    console.error('Failed to install CLI:', error);
    throw error;
  }
};

export const uninstallCli = async (): Promise<string> => {
  try {
    return await invoke('uninstall_cli');
  } catch (error) {
    console.error('Failed to uninstall CLI:', error);
    throw error;
  }
};

export const checkCliUpdate = async (): Promise<BinaryUpdateInfo> => {
  try {
    return await invoke('check_cli_update');
  } catch (error) {
    console.error('Failed to check CLI update:', error);
    throw error;
  }
};

export const updateCli = async (): Promise<string> => {
  try {
    return await invoke('update_cli');
  } catch (error) {
    console.error('Failed to update CLI:', error);
    throw error;
  }
};

// Updater
export interface UpdateInfo {
  update_available: boolean;
  current_version: string;
  latest_version: string;
  release_notes: string;
  download_url: string;
  published_at: string;
}

export const checkForUpdates = async (): Promise<UpdateInfo> => {
  try {
    return await invoke('check_for_updates');
  } catch (error) {
    console.error('Failed to check for updates:', error);
    throw error;
  }
};

export const getCurrentVersion = async (): Promise<string> => {
  try {
    return await invoke('get_current_version');
  } catch (error) {
    console.error('Failed to get current version:', error);
    throw error;
  }
};

// Terminal
export const spawnTerminal = async (id: string, cols: number, rows: number, cwd?: string): Promise<void> => {
  try {
    await invoke('spawn_terminal', { id, cols, rows, cwd });
  } catch (error) {
    console.error('Failed to spawn terminal:', error);
    throw error;
  }
};

export const writeTerminal = async (id: string, data: string): Promise<void> => {
  try {
    await invoke('write_terminal', { id, data });
  } catch (error) {
    console.error('Failed to write to terminal:', error);
    throw error;
  }
};

export const resizeTerminal = async (id: string, cols: number, rows: number): Promise<void> => {
  try {
    await invoke('resize_terminal', { id, cols, rows });
  } catch (error) {
    console.error('Failed to resize terminal:', error);
    throw error;
  }
};

export const closeTerminal = async (id: string): Promise<void> => {
  try {
    await invoke('close_terminal', { id });
  } catch (error) {
    console.error('Failed to close terminal:', error);
    throw error;
  }
};

// Workspace Settings
import { load } from '@tauri-apps/plugin-store';

export const getWorkspacePath = async (): Promise<string | null> => {
  try {
    const store = await load('.settings.json', { autoSave: false, defaults: { workspacePath: '' } });
    const path = await store.get<string>('workspacePath');
    return path || null;
  } catch (error) {
    console.error('Failed to get workspace path from store:', error);
    return null;
  }
};

// Tunneling
export interface TunnelResponse {
  success: boolean;
  message: string;
  url: string | null;
}

export const startTunnel = async (domain: string, port: number, authToken: string): Promise<TunnelResponse> => {
  try {
    return await invoke('start_tunnel', { domain, port, authToken });
  } catch (error) {
    console.error('Failed to start tunnel:', error);
    throw error;
  }
};

export const stopTunnel = async (): Promise<TunnelResponse> => {
  try {
    return await invoke('stop_tunnel');
  } catch (error) {
    console.error('Failed to stop tunnel:', error);
    throw error;
  }
};

export const getTunnelUrl = async (): Promise<string> => {
  try {
    return await invoke('get_tunnel_url');
  } catch (error) {
    console.error('Failed to get tunnel url:', error);
    throw error;
  }
};
