import { invoke } from '@tauri-apps/api/tauri';

export interface Server {
    id: string;
    hostname: string;
    ip: string;
}

export const discoverServers = async (): Promise<Server[]> => {
    try {
        return await invoke('discover_servers');
    } catch (error) {
        console.error('Failed to discover servers:', error);
        return [];
    }
};

export const connectToServer = async (server: Server): Promise<boolean> => {
    try {
        await invoke('connect_to_server', { server });
        return true;
    } catch (error) {
        console.error('Failed to connect to server:', error);
        return false;
    }
};