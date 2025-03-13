import { useState } from 'react';

type ConnectionStatusType = 'idle' | 'connecting' | 'connected' | 'failed';

interface ConnectionStatusProps {
    status: ConnectionStatusType;
}

const ConnectionStatus = ({ status }: ConnectionStatusProps) => {
    const [showDetails, setShowDetails] = useState(false);

    if (status === 'idle') {
        return <div className="h-12"></div>; // Empty space placeholder
    }

    return (
        <div className="w-full max-w-md mx-auto px-4 py-3 mt-4">
            {status === 'connecting' && (
                <div className="flex items-center text-gray-600 dark:text-gray-300">
                    <svg className="animate-spin -ml-1 mr-3 h-5 w-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    <span>Connecting to server...</span>
                </div>
            )}

            {status === 'connected' && (
                <div className="flex items-center text-green-600 dark:text-green-400 transition-opacity duration-300 animate-fadeIn">
                    <svg className="h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                        <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                    </svg>
                    <span>Connected to server.</span>
                </div>
            )}

            {status === 'failed' && (
                <div className="space-y-2">
                    <div className="flex items-center text-red-500 dark:text-red-400">
                        <svg
                            className="h-5 w-5 mr-2 cursor-pointer"
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 20 20"
                            fill="currentColor"
                            onClick={() => setShowDetails(!showDetails)}
                        >
                            <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                        </svg>
                        <span>Failed to connect to server.</span>
                        <button
                            className="ml-auto text-sm bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400 px-3 py-1 rounded-md hover:bg-red-200 dark:hover:bg-red-800/30 transition-colors"
                        >
                            Retry
                        </button>
                    </div>

                    {showDetails && (
                        <div className="mt-2 p-3 bg-gray-100 dark:bg-gray-800 rounded-md text-sm text-gray-700 dark:text-gray-300 font-mono">
                            <p>Error connecting to server. Connection timed out after 30 seconds.</p>
                            <p className="text-xs mt-1 text-gray-500 dark:text-gray-400">Error code: TIMEOUT_ERR</p>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
};

export default ConnectionStatus;
