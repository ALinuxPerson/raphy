import { useState } from "react";
import Header from "../components/main/Header";
import ServerStatus from "../components/main/ServerStatus";
import ConfigSection from "../components/main/ConfigSection";
import ControlButtons from "../components/main/ControlButtons";
import Console from "../components/main/Console";
import {ClientMode, restartServer, startServer, stopServer} from "../utils/server.ts";

const MainPage = (clientMode: ClientMode) => {
    const [showConsole, setShowConsole] = useState(false);
    const [serverStatus, setServerStatus] = useState<'stopped' | 'running' | 'restarting'>('stopped');
    const [isConfigMissing, setIsConfigMissing] = useState(true);

    const toggleConsole = () => {
        setShowConsole(!showConsole);
    };

    const handleStart = async () => {
        try {
            await startServer();
        } catch (error) {
            console.error("Failed to start the server: ", error);
            return;
        }

        setServerStatus('running');
    };

    const handleStop = async () => {
        try {
            await stopServer();
        } catch (error) {
            console.error("Failed to stop the server: ", error);
            return;
        }
        setServerStatus('stopped');
    };

    const handleRestart = async () => {
        setServerStatus('restarting');

        try {
            await restartServer();
        } catch (error) {
            console.error("Failed to restart the server: ", error);
            return;
        }

        setServerStatus('running');
    };

    return (
        <div className="flex flex-col h-screen bg-white dark:bg-gray-900 text-black dark:text-white">
            <Header />

            <div className="flex-1 container mx-auto px-4 py-6 overflow-hidden flex flex-col">
                <ServerStatus status={serverStatus} />

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
                        onStart={handleStart}
                        onStop={handleStop}
                        onRestart={handleRestart}
                        serverStatus={serverStatus}
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