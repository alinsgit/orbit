import { invoke } from '@tauri-apps/api/core'

// ── Types ──

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  size: number
  /** Unix mtime in seconds (0 if unknown). */
  modified: number
  /** Unix permission bits (0 if unknown). */
  permissions: number
}

export interface DirListing {
  path: string
  entries: FileEntry[]
}

// ── Local Filesystem ──

export const localListDir = async (path: string): Promise<DirListing> => {
  return await invoke('local_list_dir', { path })
}

export const localMkdir = async (path: string): Promise<void> => {
  return await invoke('local_mkdir', { path })
}

export const localDelete = async (path: string): Promise<void> => {
  return await invoke('local_delete', { path })
}

export const localRename = async (from: string, to: string): Promise<void> => {
  return await invoke('local_rename', { from, to })
}

export const localHomeDir = async (): Promise<string> => {
  return await invoke('local_home_dir')
}

// ── SFTP Browsing ──

export const sftpListDir = async (
  connection: string,
  path: string,
): Promise<DirListing> => {
  return await invoke('sftp_list_dir', { connection, path })
}

export const sftpMkdir = async (connection: string, path: string): Promise<void> => {
  return await invoke('sftp_mkdir', { connection, path })
}

export const sftpDelete = async (
  connection: string,
  path: string,
  isDir: boolean,
): Promise<void> => {
  return await invoke('sftp_delete', { connection, path, isDir })
}

export const sftpRename = async (
  connection: string,
  from: string,
  to: string,
): Promise<void> => {
  return await invoke('sftp_rename', { connection, from, to })
}

export const sftpDownloadPath = async (
  connection: string,
  remotePath: string,
  localPath: string,
): Promise<string> => {
  return await invoke('sftp_download_path', { connection, remotePath, localPath })
}

export const sftpUploadPath = async (
  connection: string,
  localPath: string,
  remotePath: string,
): Promise<string> => {
  return await invoke('sftp_upload_path', { connection, localPath, remotePath })
}

export const sftpDisconnect = async (connection: string): Promise<void> => {
  return await invoke('sftp_disconnect', { connection })
}

// ── Interactive SSH Terminal ──

export const sshSpawnTerminal = async (
  connection: string,
  id: string,
  cols: number,
  rows: number,
): Promise<void> => {
  return await invoke('ssh_spawn_terminal', { connection, id, cols, rows })
}

export const sshWriteTerminal = async (id: string, data: string): Promise<void> => {
  return await invoke('ssh_write_terminal', { id, data })
}

export const sshResizeTerminal = async (
  id: string,
  cols: number,
  rows: number,
): Promise<void> => {
  return await invoke('ssh_resize_terminal', { id, cols, rows })
}

export const sshCloseTerminal = async (id: string): Promise<void> => {
  return await invoke('ssh_close_terminal', { id })
}
