import { invoke } from '@tauri-apps/api/core'

export interface Server {
    addresses: string[];
    port: number;

    // Client-side properties for UI display
    id?: string;      // Will store the full name
    hostname?: string; // Will store extracted hostname
    ip?: string;      // Will store the primary IP for display
}

export const connectToServer = async (fullName: string): Promise<void> => {
    await invoke('connect_to_server', {
        by: {
            FullName: fullName
        }
    });
};

// New function to connect to a manually specified server
export const connectToServerByAddress = async (ip: string, port: number): Promise<void> => {
    const socketAddress = `${ip}:${port}`;
    await invoke('connect_to_server', {
        by: {
            SocketAddress: socketAddress
        }
    });
};

// Enum for client mode to match Rust's ClientMode enum
export enum ClientMode {
  Local = 'Local',
  Remote = 'Remote'
}

// Get the current client mode (Local or Remote)
export const clientMode = async (): Promise<ClientMode> => {
    return await invoke('client_mode') as ClientMode;
};

export enum JavaPathKind {
    AutoDetect = 'AutoDetect',
    Custom = 'Custom'
}

export enum ServerArgumentsKind {
    Parsed = 'Parsed',
    Manual = 'Manual'
}

export enum UserKind {
    Current = 'Current',
    Specific = 'Specific'
}

export interface ParsedServerArguments {
    Parsed: string;
}

export function isParsedServerArguments(args: ServerArguments): args is ParsedServerArguments {
    return (args as ParsedServerArguments).Parsed !== undefined;
}

export interface ManualServerArguments {
    Manual: string[];
}

export function isManualServerArguments(args: ServerArguments): args is ManualServerArguments {
    return (args as ManualServerArguments).Manual !== undefined;
}

export type ServerArguments = ParsedServerArguments | ManualServerArguments;

// TypeScript equivalents to Rust structs
export interface ResolvedConfig {
    java_path: string;
    server_jar_path: string;
    arguments: ServerArguments;
    user: string | null;
}

export interface ConfigMask {
    java_path: JavaPathKind;
    arguments: ServerArgumentsKind;
    user: UserKind;
}

/**
 * Get server configuration from the backend.
 * Returns a tuple of [ResolvedConfig, ConfigMask] if config exists, or null if no config available
 */
export const getServerConfig = async (): Promise<[ResolvedConfig, ConfigMask] | null> => {
    return await invoke('get_server_config') as [ResolvedConfig, ConfigMask] | null;
};

export const updateConfig = async (config: ResolvedConfig, mask: ConfigMask): Promise<void> => {
    return await invoke('update_config', { config, mask });
}

export const stopServer = async (): Promise<void> => {
    await invoke('stop_server');
}

export const startServer = async (): Promise<void> => {
    await invoke('start_server');
}

export const restartServer = async (): Promise<void> => {
    await invoke('restart_server');
}
