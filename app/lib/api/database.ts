import { invoke } from '@tauri-apps/api/core';

// Database Tools Types
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

// Database Tools API
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

// Backup/Import API
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

// PostgreSQL Backup/Import
export const pgExportDatabase = async (database: string, outputPath: string): Promise<string> => {
  try {
    return await invoke('pg_export_database', { database, outputPath });
  } catch (error) {
    console.error('Failed to export PostgreSQL database:', error);
    throw error;
  }
};

export const pgImportSql = async (database: string, sqlPath: string): Promise<string> => {
  try {
    return await invoke('pg_import_sql', { database, sqlPath });
  } catch (error) {
    console.error('Failed to import SQL into PostgreSQL:', error);
    throw error;
  }
};

// MongoDB
export const mongoListDatabases = async (): Promise<string[]> => {
  return await invoke('mongo_list_databases');
};

export const mongoListCollections = async (database: string): Promise<string[]> => {
  return await invoke('mongo_list_collections', { database });
};

export const mongoDbStats = async (database: string): Promise<string> => {
  return await invoke('mongo_db_stats', { database });
};

export const mongoDropDatabase = async (database: string): Promise<string> => {
  return await invoke('mongo_drop_database', { database });
};

export const mongoRunCommand = async (database: string, jsCommand: string): Promise<string> => {
  return await invoke('mongo_run_command', { database, jsCommand });
};
