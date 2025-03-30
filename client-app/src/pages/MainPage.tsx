import {useEffect, useState} from "react";
import Header from "../components/main/Header";
import ServerStatus from "../components/main/ServerStatus";
import ConfigSection from "../components/main/ConfigSection";
import ControlButtons from "../components/main/ControlButtons";
import Console from "../components/main/Console";
import {
    ClientMode,
    getServerStateKind, Operation,
    getServerState,
    restartServer,
    ServerState,
    ServerStateKind,
    startServer,
    stopServer
} from "../utils/server.ts";
import {listen} from "@tauri-apps/api/event";

// Show a toast notification (in Apple style)
const showNotification = (title: string, message: string, type: 'info' | 'warning' | 'error') => {
    // Create and show a notification element
    const notificationContainer = document.createElement('div');
    notificationContainer.className = `fixed top-4 right-4 p-4 rounded-lg shadow-lg animate-fadeIn z-50 ${
        type === 'error' ? 'bg-red-50 border border-red-200 dark:bg-red-900/20 dark:border-red-800' :
            type === 'warning' ? 'bg-yellow-50 border border-yellow-200 dark:bg-yellow-900/20 dark:border-yellow-800' :
                'bg-blue-50 border border-blue-200 dark:bg-blue-900/20 dark:border-blue-800'
    }`;

    const icon = type === 'error' ?
        '<svg class="w-5 h-5 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>' :
        type === 'warning' ?
            '<svg class="w-5 h-5 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>' :
            '<svg class="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>';

    notificationContainer.innerHTML = `
            <div class="flex items-start">
                <div class="flex-shrink-0">
                    ${icon}
                </div>
                <div class="ml-3 w-0 flex-1">
                    <p class="text-sm font-medium ${
        type === 'error' ? 'text-red-800 dark:text-red-400' :
            type === 'warning' ? 'text-yellow-800 dark:text-yellow-400' :
                'text-blue-800 dark:text-blue-400'
    }">${title}</p>
                    <p class="mt-1 text-sm ${
        type === 'error' ? 'text-red-700 dark:text-red-300' :
            type === 'warning' ? 'text-yellow-700 dark:text-yellow-300' :
                'text-blue-700 dark:text-blue-300'
    }">${message}</p>
                </div>
                <div class="ml-4 flex-shrink-0 flex">
                    <button class="inline-flex text-gray-400 hover:text-gray-500">
                        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
            </div>
        `;

    document.body.appendChild(notificationContainer);

    // Attach close event
    const closeButton = notificationContainer.querySelector('button');
    if (closeButton) {
        closeButton.addEventListener('click', () => {
            notificationContainer.classList.add('animate-fadeOut');
            setTimeout(() => {
                notificationContainer.remove();
            }, 300);
        });
    }

    // Auto-remove after 5 seconds
    setTimeout(() => {
        if (document.body.contains(notificationContainer)) {
            notificationContainer.classList.add('animate-fadeOut');
            setTimeout(() => {
                if (document.body.contains(notificationContainer)) {
                    notificationContainer.remove();
                }
            }, 300);
        }
    }, 5000);
};

const MainPage = (clientMode: ClientMode) => {
    const [showConsole, setShowConsole] = useState(false);
    const [operationInProgress, setOperationInProgress] = useState<Operation | null>(null);
    const [serverStateKind, setServerStateKind] = useState<ServerStateKind>("Stopped");
    const [isConfigMissing, setIsConfigMissing] = useState(true);
    const [isLoading, setIsLoading] = useState(true);

    const toggleConsole = () => {
        setShowConsole(!showConsole);
    };

    // Get initial server state on component mount
    useEffect(() => {
        const fetchInitialState = async () => {
            try {
                setIsLoading(true);
                const initialState = await getServerState();
                setServerStateKind(getServerStateKind(initialState));
            } catch (error) {
                console.error("Failed to fetch initial server state:", error);
                showNotification(
                    "Error",
                    "Failed to retrieve server state. Please try refreshing the page.",
                    'error'
                );
            } finally {
                setIsLoading(false);
            }
        };

        fetchInitialState();
    }, []);

    // Listen for server state and operation events
    useEffect(() => {
        const operationRequestedUnlisten = listen("operation-requested", (event) => {
            const [operation, _] = event.payload as [Operation, string];
            setOperationInProgress(operation);
        });

        const operationPerformedUnlisten = listen("operation-performed", (_event) => {
            setOperationInProgress(null);
        });

        const operationFailedUnlisten = listen("operation-failed", (event) => {
            const [operation, _, error] = event.payload as [string, string, string];

            if (operation === "Start") {
                showNotification("Operation Failed", `Failed to start server.\n${error}`, 'error');
            } else if (operation === "Stop") {
                showNotification("Operation Failed", `Failed to stop server.\n${error}`, 'error');
            } else if (operation === "Restart") {
                showNotification("Operation Failed", `Failed to restart server.\n${error}`, 'error');
            }

            setOperationInProgress(null);
        });

        const serverStateUpdatedUnlisten = listen<ServerState>("server-state-updated", (event) => {
            const state = event.payload;
            setServerStateKind(getServerStateKind(state));
        });

        return () => {
            operationRequestedUnlisten.then(fn => fn());
            operationPerformedUnlisten.then(fn => fn());
            operationFailedUnlisten.then(fn => fn());
            serverStateUpdatedUnlisten.then(fn => fn());
        };
    }, []);

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-screen bg-white dark:bg-gray-900">
                <div className="text-center">
                    <svg className="animate-spin mx-auto h-10 w-10 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    <p className="mt-4 text-gray-600 dark:text-gray-300">Loading server status...</p>
                </div>
            </div>
        );
    }

    return (
        <div className="flex flex-col h-screen bg-white dark:bg-gray-900 text-black dark:text-white">
            <Header />

            <div className="flex-1 container mx-auto px-4 py-6 overflow-hidden flex flex-col">
                <ServerStatus serverStateKind={serverStateKind}/>

                <div className="flex-1 flex flex-col md:flex-row mt-6 gap-6">
                    <div className="flex-1">
                        <ConfigSection clientMode={clientMode} isConfigMissing={isConfigMissing} setIsConfigMissing={setIsConfigMissing} />
                    </div>

                    {showConsole && (
                        <div className="flex-1 mt-6 md:mt-0">
                            <Console />
                        </div>
                    )}
                </div>

                <div className="mt-6 flex justify-between items-center">
                    <ControlButtons
                        isConfigMissing={isConfigMissing}
                        onStart={() => void startServer()}
                        onStop={() => void stopServer()}
                        onRestart={() => void restartServer()}
                        serverStateKind={serverStateKind}
                        operationInProgress={operationInProgress}
                    />

                    <button
                        onClick={toggleConsole}
                        className="flex items-center justify-center w-10 h-10 rounded-full bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                        aria-label="Toggle console"
                    >
                        <svg className="w-5 h-5 text-gray-700 dark:text-gray-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M8 9l3 3-3 3M13 15h3" strokeLinecap="round" strokeLinejoin="round" />
                            <rect x="2" y="4" width="20" height="16" rx="2" strokeLinecap="round" strokeLinejoin="round" />
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    );
};

export default MainPage;