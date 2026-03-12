import { invoke } from '@tauri-apps/api/core';

export interface DeployConnection {
  name: string;
  protocol: 'SSH' | 'SFTP' | 'FTP';
  host: string;
  port: number;
  username: string;
  auth: 'Password' | { KeyFile: string };
  remote_path: string;
}

export interface DeployManifest {
  timestamp: string;
  domain: string;
  connection: string;
  files: { path: string; hash: string; size: number }[];
  status: 'InProgress' | 'Completed' | { Failed: string };
}

export const deployListConnections = async (domain: string): Promise<DeployConnection[]> => {
  try {
    return await invoke('deploy_list_connections', { domain });
  } catch (error) {
    console.error('Failed to list deploy connections:', error);
    throw error;
  }
};

export const deployAddConnection = async (
  domain: string,
  connection: DeployConnection,
  password?: string,
): Promise<string> => {
  try {
    return await invoke('deploy_add_connection', { domain, connection, password });
  } catch (error) {
    console.error('Failed to add deploy connection:', error);
    throw error;
  }
};

export const deployRemoveConnection = async (
  domain: string,
  connName: string,
): Promise<string> => {
  try {
    return await invoke('deploy_remove_connection', { domain, connName });
  } catch (error) {
    console.error('Failed to remove deploy connection:', error);
    throw error;
  }
};

export const deployTestConnection = async (
  domain: string,
  connName: string,
): Promise<string> => {
  try {
    return await invoke('deploy_test_connection', { domain, connName });
  } catch (error) {
    console.error('Failed to test deploy connection:', error);
    throw error;
  }
};

export const deploySshExecute = async (
  domain: string,
  connName: string,
  command: string,
): Promise<string> => {
  try {
    return await invoke('deploy_ssh_execute', { domain, connName, command });
  } catch (error) {
    console.error('Failed to execute SSH command:', error);
    throw error;
  }
};

export const deploySync = async (
  domain: string,
  connName: string,
  sitePath: string,
): Promise<DeployManifest> => {
  try {
    return await invoke('deploy_sync', { domain, connName, sitePath });
  } catch (error) {
    console.error('Failed to deploy:', error);
    throw error;
  }
};

export const deployGetStatus = async (
  domain: string,
  connName: string,
): Promise<DeployManifest | null> => {
  try {
    return await invoke('deploy_get_status', { domain, connName });
  } catch (error) {
    console.error('Failed to get deploy status:', error);
    throw error;
  }
};
