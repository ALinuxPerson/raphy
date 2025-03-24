import React from 'react';
import {useConnection} from "../contexts/ConnectionContext.tsx";

const ConnectionFailureOverlay: React.FC = () => {
    const { status, tryReconnect } = useConnection();

    if (status === 'connected') return null;

    return (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 max-w-md w-full mx-4 animate-fadeIn">
                <div className="text-center">
                    <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 mb-4">
                        <svg className="w-8 h-8 text-red-600 dark:text-red-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                    </div>

                    <h2 className="text-xl font-semibold text-white mb-2">Connection Lost</h2>
                    <p className="text-gray-600 dark:text-gray-400 mb-6">
                        The connection to the server has been lost. This could be due to network issues or the server may have stopped responding.
                    </p>

                    <div className="flex justify-center">
                        <button
                            onClick={tryReconnect}
                            disabled={status === 'reconnecting'}
                            className={`px-4 py-2 rounded-md text-white font-medium transition-all flex items-center justify-center min-w-[120px] ${
                                status === 'reconnecting'
                                    ? 'bg-blue-400 cursor-not-allowed'
                                    : 'bg-blue-500 hover:bg-blue-600 active:bg-blue-700'
                            }`}
                        >
                            {status === 'reconnecting' ? (
                                <>
                                    <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    Reconnecting...
                                </>
                            ) : (
                                'Reconnect'
                            )}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default ConnectionFailureOverlay;