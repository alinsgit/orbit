import { invoke } from '@tauri-apps/api/core';

export type WebServer = 'nginx' | 'apache';

export interface Site {
  domain: string;
  path: string;
  port: number;
  php_version?: string;
  php_port?: number;
  ssl_enabled?: boolean;
  template?: 'http' | 'laravel' | 'wordpress' | 'litecart' | 'static' | 'nextjs' | 'astro' | 'nuxt' | 'vue' | 'django' | 'sveltekit' | 'remix';
  web_server?: WebServer;
  dev_port?: number;
  dev_command?: string;
}

export interface SiteWithStatus extends Site {
  dev_port?: number;
  dev_command?: string;
  created_at?: string;
  config_valid: boolean;
  warning?: string;
  web_server: WebServer;
}

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

export const startSiteApp = async (domain: string): Promise<number> => {
  return await invoke('start_site_app', { domain });
};

export const stopSiteApp = async (domain: string): Promise<void> => {
  return await invoke('stop_site_app', { domain });
};

export const getSiteAppStatus = async (domain: string): Promise<string> => {
  return await invoke('get_site_app_status', { domain });
};

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

export const readSiteConfig = async (domain: string): Promise<string> => {
  return await invoke('read_site_config', { domain });
};

export const writeSiteConfig = async (domain: string, content: string): Promise<string> => {
  return await invoke('write_site_config', { domain, content });
};

export const nginxStatus = async (): Promise<boolean> => {
  try {
    return await invoke('nginx_status');
  } catch (error) {
    console.error('Failed to get nginx status:', error);
    throw error;
  }
};

export const scaffoldProject = async (
  projectType: string,
  projectName: string,
  workspacePath: string
): Promise<string> => {
  try {
    return await invoke('scaffold_project', { projectType, projectName, workspacePath });
  } catch (error) {
    console.error(`Failed to scaffold ${projectType} project:`, error);
    throw error;
  }
};

export const scaffoldBasicProject = async (path: string, template: string): Promise<string> => {
  try {
    return await invoke('scaffold_basic_project', { path, template });
  } catch (error) {
    console.error(`Failed to scaffold basic ${template} project:`, error);
    throw error;
  }
};

// Blueprints

export interface Blueprint {
  name: string;
  description: string;
  services: string[];
  template: string;
  scaffold: string[];
  php_extensions: string[];
  env_template?: string;
  dev_command?: string;
}

export interface BlueprintResult {
  site: SiteWithStatus;
  scaffold_commands: string[];
  dev_command?: string;
  warnings: string[];
}

export const listBlueprints = async (): Promise<Blueprint[]> => {
  return await invoke('list_blueprints');
};

export const createFromBlueprint = async (
  blueprint: string,
  domain: string,
  path: string,
  phpVersion?: string
): Promise<BlueprintResult> => {
  return await invoke('create_from_blueprint', {
    blueprint,
    domain,
    path,
    phpVersion,
  });
};
