import "./style.css";
import SetupPage from "./pages/SetupPage.tsx";
import { useState, useEffect } from "react";
import MainPage from "./pages/MainPage.tsx";
import {clientConnectionActive, ClientMode} from "./utils/server.ts";
import { clientModeStore } from "./utils/clientModeStore.ts";
import { ConnectionProvider } from "./contexts/ConnectionContext.tsx";
import ConnectionFailureOverlay from "./components/ConnectionFailureOverlay.tsx";

function App() {
    const [currentPage, setCurrentPage] = useState<"setup" | "main">("setup");
    const [mode, setMode] = useState<ClientMode | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const init = async () => {
            clientModeStore.getMode().then(clientMode => {
                setMode(clientMode);

                clientConnectionActive().then(isActive => {
                    if (isActive) {
                        setCurrentPage("main");
                    } else {
                        setCurrentPage("setup");
                    }
                })

                setLoading(false);
            });
        }

        init();
    }, []);

    const navigateToMainPage = () => { setCurrentPage("main"); };

    if (loading) {
        return <div>Loading...</div>;
    }

    return (
        <ConnectionProvider>
            <div>
                {currentPage === "setup"
                    ? <SetupPage navigateToMainPage={navigateToMainPage} />
                    : <MainPage clientMode={mode!} />}
            </div>
            <ConnectionFailureOverlay />
        </ConnectionProvider>
    );
}

export default App;