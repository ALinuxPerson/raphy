import React from "react";
import {Operation, ServerStateKind} from "../../utils/server.ts";

interface ControlButtonProps {
    isConfigMissing: boolean;
    onStart: () => void;
    onStop: () => void;
    onRestart: () => void;
    serverStateKind: ServerStateKind;
    operationInProgress: Operation | null;
}

const ControlButtons: React.FC<ControlButtonProps> = ({
                                                          isConfigMissing,
                                                          onStart,
                                                          onStop,
                                                          onRestart,
                                                          serverStateKind,
                                                          operationInProgress
                                                      }) => {
    // Determine if any operation is currently in progress
    const isOperationInProgress = operationInProgress !== null;

    // Button states
    const isStartDisabled = isConfigMissing || isOperationInProgress || serverStateKind === "Started";
    const isStopDisabled = isConfigMissing || isOperationInProgress || serverStateKind === "Stopped";
    const isRestartDisabled = isConfigMissing || isOperationInProgress || serverStateKind === "Stopped";

    // Button titles
    const startTitle = isConfigMissing ? "Configuration is missing" :
        isOperationInProgress ? "Operation in progress" :
            serverStateKind === "Started" ? "Server is already running" : "Start server";

    const stopTitle = isConfigMissing ? "Configuration is missing" :
        isOperationInProgress ? "Operation in progress" :
            serverStateKind === "Stopped" ? "Server is already stopped" : "Stop server";

    const restartTitle = isConfigMissing ? "Configuration is missing" :
        isOperationInProgress ? "Operation in progress" :
            serverStateKind === "Stopped" ? "Server is not running" : "Restart server";

    return (
        <div className="flex space-x-4">
            {/* Start Button */}
            <button
                onClick={onStart}
                disabled={isStartDisabled}
                className={`flex items-center px-4 py-2 rounded-md text-sm font-medium transition-all duration-200
                    ${isStartDisabled
                    ? 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 cursor-not-allowed'
                    : 'bg-green-500 text-white hover:bg-green-600 active:bg-green-700'}`}
                title={startTitle}
            >
                <svg className="w-4 h-4 mr-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                     strokeLinecap="round" strokeLinejoin="round">
                    <polygon points="5 3 19 12 5 21 5 3"></polygon>
                </svg>
                Start
            </button>

            {/* Stop Button */}
            <button
                onClick={onStop}
                disabled={isStopDisabled}
                className={`flex items-center px-4 py-2 rounded-md text-sm font-medium transition-all duration-200
                    ${isStopDisabled
                    ? 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 cursor-not-allowed'
                    : 'bg-red-500 text-white hover:bg-red-600 active:bg-red-700'}`}
                title={stopTitle}
            >
                <svg className="w-4 h-4 mr-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                     strokeLinecap="round" strokeLinejoin="round">
                    <rect x="6" y="6" width="12" height="12"></rect>
                </svg>
                Stop
            </button>

            {/* Restart Button */}
            <button
                onClick={onRestart}
                disabled={isRestartDisabled}
                className={`flex items-center px-4 py-2 rounded-md text-sm font-medium transition-all duration-200
                    ${isRestartDisabled
                    ? 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 cursor-not-allowed'
                    : 'bg-yellow-500 text-white hover:bg-yellow-600 active:bg-yellow-700'}`}
                title={restartTitle}
            >
                <svg className="w-4 h-4 mr-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
                     strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2"></path>
                </svg>
                Restart
            </button>
        </div>
    );
};

export default ControlButtons;