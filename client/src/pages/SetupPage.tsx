import { useState, useEffect } from "react";
import ServerList from "../components/setup/ServerList.tsx";
import ConnectionStatus from "../components/setup/ConnectionStatus.tsx";
import NavigationButtons from "../components/setup/NavigationButtons.tsx";

const SetupPage = () => {
    const [servers, setServers] = useState([
        { id: '1', hostname: 'Server 1', ip: '192.168.1.1' },
        { id: '2', hostname: 'Server 2', ip: '192.168.1.2' },
    ])
    const [selectedServer, setSelectedServer] = useState(null)
    const [connectionStatus, setConnectionStatus] = useState("idle")

    const handleConnect = async () => {
        if (!selectedServer) return;

        setConnectionStatus('connecting');

        try {
            // Use Tauri API to connect to the server
            // await window.__TAURI__.invoke('connect_to_server', { server: selectedServer })

            setConnectionStatus('connected');
            // Save server to configuration
        } catch (error) {
            setConnectionStatus('failed');
            console.error(error);
        }
    };

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
                servers={servers}
                selectedServer={selectedServer}
                onSelectServer={setSelectedServer}
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