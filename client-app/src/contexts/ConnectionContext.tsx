import React, { createContext, useContext, useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

type ConnectionStatus = 'connected' | 'disconnected' | 'reconnecting';

interface ConnectionContextType {
    status: ConnectionStatus;
    tryReconnect: () => Promise<void>;
}

const ConnectionContext = createContext<ConnectionContextType>({
    status: 'connected',
    tryReconnect: async () => {}
});

export const useConnection = () => useContext(ConnectionContext);

export const ConnectionProvider: React.FC<{children: React.ReactNode}> = ({ children }) => {
    const [status, setStatus] = useState<ConnectionStatus>('connected');

    // Listen for connection failure events
    useEffect(() => {
        const unlisten = listen('connection-failure', () => {
            setStatus('disconnected');
        });

        return () => {
            unlisten.then(unlistenFn => unlistenFn());
        };
    }, []);

    const tryReconnect = async () => {
        setStatus('reconnecting');

        try {
            // Implement reconnection logic here based on ClientMode
            // This is a placeholder - you'll need to implement the actual reconnection
            // logic based on whether the client is local or remote

            // For demo purposes just switch back to connected after a delay
            await new Promise(resolve => setTimeout(resolve, 2000));
            setStatus('connected');
        } catch (error) {
            console.error('Reconnection failed:', error);
            setStatus('disconnected');
        }
    };

    return (
        <ConnectionContext.Provider value={{ status, tryReconnect }}>
            {children}
        </ConnectionContext.Provider>
    );
};