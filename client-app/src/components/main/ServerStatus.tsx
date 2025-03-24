import {ServerStateKind} from "../../utils/server.ts";

interface ServerStatusProps {
    serverStateKind: ServerStateKind;
}

const ServerStatus: React.FC<ServerStatusProps> = ({ serverStateKind }) => {
    const getStatusInfo = () => {
        switch (serverStateKind) {
            case "Started":
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
            case "Stopped":
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
