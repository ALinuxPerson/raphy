import React from 'react';
import { Server } from '../../utils/server';

interface ServerListProps {
    servers: Server[];
    selectedServer: Server | null;
    onSelectServer: (server: Server) => void;
    onRefresh?: () => void;
    disabled: boolean;
}

const ServerList: React.FC<ServerListProps> = ({
                                                   servers,
                                                   selectedServer,
                                                   onSelectServer,
                                                   onRefresh,
                                                   disabled
                                               }) => {
    return (
        <div className={`flex-1 mx-auto w-full max-w-md p-4 ${disabled ? 'opacity-50' : ''}`}>
            {servers.length > 0 ? (
                <ul className="space-y-2">
                    {servers.map(server => (
                        <li
                            key={server.id}
                            onClick={() => !disabled && onSelectServer(server)}
                            className={`
                p-4 rounded-lg border flex items-center cursor-pointer 
                transition-all duration-200 
                ${selectedServer?.id === server.id
                                ? 'bg-blue-50 border-blue-500 dark:bg-blue-900/20 dark:border-blue-400'
                                : 'bg-white border-gray-200 hover:border-gray-300 dark:bg-gray-800 dark:border-gray-700 dark:hover:border-gray-600'}
                ${disabled ? 'cursor-default' : 'hover:shadow-sm'}
              `}
                        >
                            <div className="rounded-full bg-gray-100 dark:bg-gray-700 p-2 mr-4">
                                <svg className="w-6 h-6 text-gray-600 dark:text-gray-300" fill="none"
                                     viewBox="0 0 24 24" stroke="currentColor">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                                          d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                                </svg>
                            </div>
                            <div>
                                <p className="font-medium">{server.hostname}</p>
                                <p className="text-sm text-gray-500 dark:text-gray-400">{server.ip}</p>
                            </div>
                        </li>
                    ))}
                </ul>
            ) : (
                <div className="text-center py-10">
                    <p className="text-gray-500 dark:text-gray-400">No servers detected</p>
                    <button
                        className="mt-4 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
                        onClick={onRefresh}
                    >
                        Refresh
                    </button>
                </div>
            )}

            {servers.length > 0 && (
                <div className="text-center mt-4">
                    <button
                        className="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
                        onClick={onRefresh}
                        disabled={disabled}
                    >
                        Refresh Server List
                    </button>
                </div>
            )}
        </div>
    );
};

export default ServerList;