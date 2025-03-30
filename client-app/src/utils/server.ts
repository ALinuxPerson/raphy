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

export const clientConnectionActive = async (): Promise<boolean> => {
    return await invoke('client_connection_active') as boolean;
}

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

export enum ArgumentsKind {
    Parsed = 'Parsed',
    Manual = 'Manual'
}

export enum UserKind {
    Current = 'Current',
    Specific = 'Specific'
}

export interface ParsedArguments {
    Parsed: string;
}

export function isParsedArguments(args: Arguments): args is ParsedArguments {
    return (args as ParsedArguments).Parsed !== undefined;
}

export interface ManualArguments {
    Manual: string[];
}

export function isManualArguments(args: Arguments): args is ManualArguments {
    return (args as ManualArguments).Manual !== undefined;
}

export type Arguments = ParsedArguments | ManualArguments;

export interface ResolvedConfig {
    java_path: string;
    server_jar_path: string;
    java_arguments: Arguments;
    server_arguments: Arguments;
    user: string | null;
}

export interface ConfigMask {
    java_path: JavaPathKind;
    arguments: ArgumentsKind;
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

export type ServerState = "Started" | StoppedServerState;

export interface StoppedServerState {
    Stopped?: ExitStatus;
}

export enum ExitStatus {
    Success = 'Success',
    Failure = 'Failure'
}

export type ServerStateKind = "Started" | "Stopped";

export function getServerStateKind(state: ServerState): ServerStateKind {
    if (state === 'Started') {
        return "Started";
    } else {
        return "Stopped";
    }
}

export enum Operation {
    Start = 'Start',
    Stop = 'Stop',
    Restart = 'Restart'
}

export const getServerState = async (): Promise<ServerState> => {
    return await invoke('get_server_state') as ServerState;
}