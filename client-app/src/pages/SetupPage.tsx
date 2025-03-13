import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { connectToServer, Server } from "../utils/server";
import ServerList from "../components/setup/ServerList.tsx";
import ConnectionStatus from "../components/setup/ConnectionStatus.tsx";
import NavigationButtons from "../components/setup/NavigationButtons.tsx";

const SetupPage = () => {
    const [servers, setServers] = useState<Record<string, Server>>({});
    const [selectedServerId, setSelectedServerId] = useState<string | null>(null);
    const [connectionStatus, setConnectionStatus] = useState("idle");

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
                    ip: Array.from(server.addresses)[0] || 'Unknown'
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
        if (!selectedServerId || !servers[selectedServerId]) return;

        setConnectionStatus('connecting');

        try {
            const connected = await connectToServer(selectedServerId);

            if (connected) {
                setConnectionStatus('connected');
            } else {
                setConnectionStatus('failed');
            }
        } catch (error) {
            setConnectionStatus('failed');
            console.error(error);
        }
    };

    const handleRefresh = () => {
        // The server discovery is handled automatically by the backend
        // This is just a visual cue for the user that something is happening
        setConnectionStatus('searching');
        setTimeout(() => {
            if (connectionStatus === 'searching') {
                setConnectionStatus('idle');
            }
        }, 2000);
    };

    // Convert servers object to array for the ServerList component
    const serversArray = Object.values(servers);
    const selectedServer = selectedServerId ? servers[selectedServerId] : null;

    return (
        <div className="flex flex-col h-screen bg-white dark:bg-gray-900 text-black dark:text-white">
            {/* Header */}
            <div className="text-center py-6">
                <h1 className="text-3xl font-bold">Raphy</h1>
                <h2 className="text-xl mt-2">Setup Process</h2>
                <p className="mt-2">To begin, please select the appropriate server from the list below.</p>
            </div>

            {/* Server List */}
            <ServerList
                servers={serversArray}
                selectedServer={selectedServer}
                onSelectServer={(server) => setSelectedServerId(server.id)}
                onRefresh={handleRefresh}
                disabled={connectionStatus === 'connecting' || connectionStatus === 'connected'}
            />

            {/* Connection Status */}
            <ConnectionStatus status={connectionStatus} />

            {/* Navigation */}
            <div className="mt-auto p-4">
                <NavigationButtons
                    onConnect={handleConnect}
                    canConnect={!!selectedServer && connectionStatus !== 'connecting' && connectionStatus !== 'connected'}
                    canProceed={connectionStatus === 'connected'}
                    onProceed={() => {/* Navigate to main page */}}
                />
            </div>
        </div>
    );
}

export default SetupPage;