import { invoke } from '@tauri-apps/api/core'

export interface Server {
    addresses: string[];
    port: number;

    // Client-side properties for UI display
    id?: string;      // Will store the full name
    hostname?: string; // Will store extracted hostname
    ip?: string;      // Will store the primary IP for display
}

export const connectToServer = async (fullName: string): Promise<boolean> => {
    try {
        await invoke('connect_to_server', { fullName });
        return true;
    } catch (error) {
        console.error('Failed to connect to server:', error);
        return false;
    }
};