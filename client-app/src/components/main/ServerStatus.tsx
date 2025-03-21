import React from 'react';

interface ServerStatusProps {
    status: 'stopped' | 'running' | 'restarting';
}

const ServerStatus: React.FC<ServerStatusProps> = ({ status }) => {
    const getStatusInfo = () => {
        switch (status) {
            case 'running':
                return {
                    label: 'Running',
                    color: 'bg-green-100 text-green-800 border-green-200',
                    darkColor: 'dark:bg-green-900/20 dark:text-green-400 dark:border-green-800',
                    icon: (
                        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                        </svg>
                    )
                };
            case 'stopped':
                return {
                    label: 'Stopped',
                    color: 'bg-red-100 text-red-800 border-red-200',
                    darkColor: 'dark:bg-red-900/20 dark:text-red-400 dark:border-red-800',
                    icon: (
                        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <rect x="6" y="6" width="12" height="12" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>
                    )
                };
            case 'restarting':
                return {
                    label: 'Restarting',
                    color: 'bg-yellow-100 text-yellow-800 border-yellow-200',
                    darkColor: 'dark:bg-yellow-900/20 dark:text-yellow-400 dark:border-yellow-800',
                    icon: (
                        <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path strokeLinecap="round" strokeLinejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                    )
                };
        }
    };

    const statusInfo = getStatusInfo();

    return (
        <div className="w-full max-w-md mx-auto">
            <div className="flex justify-center">
                <div className={`inline-flex items-center px-3 py-1 rounded-full border ${statusInfo.color} ${statusInfo.darkColor}`}>
                    <span className="mr-1">{statusInfo.icon}</span>
                    <span className="text-sm font-medium">Server {statusInfo.label}</span>
                </div>
            </div>
        </div>
    );
};

export default ServerStatus;
