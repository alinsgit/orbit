import { invoke } from '@tauri-apps/api/core'

// ── Interfaces ──

export interface ServerConnection {
  name: string
  host: string
  port: number
  username: string
  auth: 'Password' | { KeyFile: string }
  protocol: 'SSH' | 'FTP'
}

export interface DeployTarget {
  connection: string
  remote_path: string
}

export interface DeployManifest {
  timestamp: string
  domain: string
  connection: string
  files: { path: string; hash: string; size: number }[]
  status: 'InProgress' | 'Completed' | { Failed: string }
}

// ── Global Connections ──

export const deployListConnections = async (): Promise<ServerConnection[]> => {
  return await invoke('deploy_list_connections')
}

export const deployAddConnection = async (
  connection: ServerConnection,
  password?: string,
): Promise<string> => {
  return await invoke('deploy_add_connection', { connection, password })
}

export const deployRemoveConnection = async (name: string): Promise<string> => {
  return await invoke('deploy_remove_connection', { name })
}

export const deployTestConnection = async (name: string): Promise<string> => {
  return await invoke('deploy_test_connection', { name })
}

// ── Site Targets ──

export const deployListTargets = async (domain: string): Promise<DeployTarget[]> => {
  return await invoke('deploy_list_targets', { domain })
}

export const deployAssignTarget = async (
  domain: string,
  connection: string,
  remotePath: string,
): Promise<string> => {
  return await invoke('deploy_assign_target', { domain, connection, remotePath })
}

export const deployUnassignTarget = async (
  domain: string,
  connection: string,
): Promise<string> => {
  return await invoke('deploy_unassign_target', { domain, connection })
}

// ── Operations ──

export const deploySync = async (
  domain: string,
  connection: string,
  sitePath: string,
): Promise<DeployManifest> => {
  return await invoke('deploy_sync', { domain, connection, sitePath })
}

export const deploySshExecute = async (
  connection: string,
  command: string,
): Promise<string> => {
  return await invoke('deploy_ssh_execute', { connection, command })
}

export const deployGetStatus = async (
  domain: string,
  connection: string,
): Promise<DeployManifest | null> => {
  return await invoke('deploy_get_status', { domain, connection })
}
