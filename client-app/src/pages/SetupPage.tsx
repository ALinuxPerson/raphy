import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { connectToServer, connectToServerByAddress, Server } from "../utils/server";
import ServerList from "../components/setup/ServerList.tsx";
import ConnectionStatus, {ConnectionStatusType} from "../components/setup/ConnectionStatus.tsx";
import NavigationButtons from "../components/setup/NavigationButtons.tsx";

interface SetupPageProps {
    navigateToMainPage: () => void;
}

const SetupPage = ({ navigateToMainPage }: SetupPageProps) => {
    const [servers, setServers] = useState<Record<string, Server>>({});
    const [selectedServerId, setSelectedServerId] = useState<string | null>(null);
    const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusType>("idle");
    const [isManualEntry, setIsManualEntry] = useState(false);
    const [manualServer, setManualServer] = useState<{
        hostname: string;
        ip: string;
        port: number;
    }>({
        hostname: "",
        ip: "",
        port: 1234, // Default port
    });

    useEffect(() => {
        // Listen for server updates from the Tauri backend
        const unlisten = listen<Record<string, Server>>("servers-updated", (event) => {
            const serversMap = event.payload;

            // Enhance server objects with display properties
            const enhancedServersMap: Record<string, Server> = {};

            Object.entries(serversMap).forEach(([fullName, server]) => {
                enhancedServersMap[fullName] = {
                    ...server,
                    id: fullName,
                    hostname: fullName.split('.')[0], // Extract hostname from fullname
                    ip: Array.from(server.addresses)[0]?.toString() || 'Unknown'
                };
            });

            setServers(enhancedServersMap);

            // If the previously selected server is no longer in the list, deselect it
            if (selectedServerId && !enhancedServersMap[selectedServerId]) {
                setSelectedServerId(null);
            }
        });

        return () => {
            unlisten.then(unlistenFn => unlistenFn());
        };
    }, [selectedServerId]);

    const handleConnect = async () => {
        if (isManualEntry) {
            setConnectionStatus('connecting');

            try {
                // Call the backend connect_to_server with socket address
                await connectToServerByAddress(manualServer.ip, manualServer.port);
                setConnectionStatus('connected');
            } catch (error) {
                setConnectionStatus('failed');
                console.error(error);
            }
        } else {
            if (!selectedServerId || !servers[selectedServerId]) return;

            setConnectionStatus('connecting');

            try {
                await connectToServer(selectedServerId);
                setConnectionStatus('connected');
            } catch (error) {
                setConnectionStatus('failed');
                console.error(error);
            }
        }
    };

    // Convert servers object to array for the ServerList component
    const serversArray = Object.values(servers);
    const selectedServer = selectedServerId ? servers[selectedServerId] : null;

    // Determine if connect button should be enabled
    const canConnect = isManualEntry
        ? (manualServer.hostname.trim() !== '' && manualServer.ip.trim() !== '')
        : (!!selectedServer && connectionStatus !== 'connecting' && connectionStatus !== 'connected');

    return (
        <div className="flex flex-col h-screen bg-white dark:bg-gray-900 text-black dark:text-white">
            {/* Header */}
            <div className="text-center py-6">
                <h1 className="text-3xl font-bold">Raphy</h1>
                <h2 className="text-xl mt-2">Setup Process</h2>
                <p className="mt-2">To begin, please select the appropriate server from the list below.</p>
            </div>

            {/* Mode Toggle */}
            <div className="flex justify-center mb-4">
                <div className="inline-flex rounded-lg border border-gray-300 dark:border-gray-700 overflow-hidden">
                    <button
                        onClick={() => setIsManualEntry(false)}
                        className={`px-4 py-2 text-sm font-medium transition-colors duration-200 ${
                            !isManualEntry
                                ? 'bg-blue-500 text-white'
                                : 'bg-transparent hover:bg-gray-100 dark:hover:bg-gray-800'
                        }`}
                    >
                        Automatic Discovery
                    </button>
                    <button
                        onClick={() => setIsManualEntry(true)}
                        className={`px-4 py-2 text-sm font-medium transition-colors duration-200 ${
                            isManualEntry
                                ? 'bg-blue-500 text-white'
                                : 'bg-transparent hover:bg-gray-100 dark:hover:bg-gray-800'
                        }`}
                    >
                        Manual Entry
                    </button>
                </div>
            </div>

            {/* Server Selection Area */}
            {isManualEntry ? (
                <div className={`flex-1 mx-auto w-full max-w-md p-4 ${connectionStatus === 'connecting' || connectionStatus === 'connected' ? 'opacity-50' : ''}`}>
                    <div className="space-y-4">
                        <div>
                            <label className="block text-sm font-medium mb-1">Hostname</label>
                            <input
                                type="text"
                                value={manualServer.hostname}
                                onChange={(e) => setManualServer({...manualServer, hostname: e.target.value})}
                                placeholder="Server Name"
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                disabled={connectionStatus === 'connecting' || connectionStatus === 'connected'}
                            />
                        </div>
                        <div>
                            <label className="block text-sm font-medium mb-1">IP Address</label>
                            <input
                                type="text"
                                value={manualServer.ip}
                                onChange={(e) => setManualServer({...manualServer, ip: e.target.value})}
                                placeholder="192.168.1.100"
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                disabled={connectionStatus === 'connecting' || connectionStatus === 'connected'}
                            />
                        </div>
                        <div>
                            <label className="block text-sm font-medium mb-1">Port</label>
                            <input
                                type="number"
                                value={manualServer.port}
                                onChange={(e) => setManualServer({...manualServer, port: parseInt(e.target.value, 10) || 1234})}
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                disabled={connectionStatus === 'connecting' || connectionStatus === 'connected'}
                            />
                        </div>
                    </div>
                </div>
            ) : (
                <ServerList
                    servers={serversArray}
                    selectedServer={selectedServer}
                    onSelectServer={(server) => setSelectedServerId(server.id || null)}
                    disabled={connectionStatus === 'connecting' || connectionStatus === 'connected'}
                />
            )}

            {/* Connection Status */}
            <ConnectionStatus status={connectionStatus} />

            {/* Navigation */}
            <div className="mt-auto p-4">
                <NavigationButtons
                    onConnect={handleConnect}
                    canConnect={canConnect}
                    canProceed={connectionStatus === 'connected'}
                    onProceed={navigateToMainPage}
                />
            </div>
        </div>
    );
}

export default SetupPage;