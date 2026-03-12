import { invoke } from '@tauri-apps/api/core';

export interface AiToolStatus {
  installed: boolean;
  path: string | null;
  version: string | null;
}

// Claude Code
export const getClaudeCodeStatus = async (): Promise<AiToolStatus> => {
  try {
    return await invoke('get_claude_code_status');
  } catch (error) {
    console.error('Failed to get Claude Code status:', error);
    throw error;
  }
};

export const installClaudeCode = async (): Promise<string> => {
  try {
    return await invoke('install_claude_code');
  } catch (error) {
    console.error('Failed to install Claude Code:', error);
    throw error;
  }
};

export const uninstallClaudeCode = async (): Promise<string> => {
  try {
    return await invoke('uninstall_claude_code');
  } catch (error) {
    console.error('Failed to uninstall Claude Code:', error);
    throw error;
  }
};

export const updateClaudeCode = async (): Promise<string> => {
  try {
    return await invoke('update_claude_code');
  } catch (error) {
    console.error('Failed to update Claude Code:', error);
    throw error;
  }
};

// Gemini CLI
export const getGeminiCliStatus = async (): Promise<AiToolStatus> => {
  try {
    return await invoke('get_gemini_cli_status');
  } catch (error) {
    console.error('Failed to get Gemini CLI status:', error);
    throw error;
  }
};

export const installGeminiCli = async (): Promise<string> => {
  try {
    return await invoke('install_gemini_cli');
  } catch (error) {
    console.error('Failed to install Gemini CLI:', error);
    throw error;
  }
};

export const uninstallGeminiCli = async (): Promise<string> => {
  try {
    return await invoke('uninstall_gemini_cli');
  } catch (error) {
    console.error('Failed to uninstall Gemini CLI:', error);
    throw error;
  }
};

export const updateGeminiCli = async (): Promise<string> => {
  try {
    return await invoke('update_gemini_cli');
  } catch (error) {
    console.error('Failed to update Gemini CLI:', error);
    throw error;
  }
};

// Context generation
export const generateAiContext = async (domain: string): Promise<string> => {
  try {
    return await invoke('generate_ai_context_cmd', { domain });
  } catch (error) {
    console.error('Failed to generate AI context:', error);
    throw error;
  }
};

// MCP config
export const setupMcpConfig = async (): Promise<string> => {
  try {
    return await invoke('setup_mcp_config');
  } catch (error) {
    console.error('Failed to setup MCP config:', error);
    throw error;
  }
};
