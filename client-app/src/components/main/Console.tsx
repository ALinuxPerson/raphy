import React, { useState, useEffect, useRef } from 'react';

type LogLevel = 'info' | 'warning' | 'error';

interface LogEntry {
    timestamp: Date;
    level: LogLevel;
    message: string;
}

const Console: React.FC = () => {
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [autoScroll, setAutoScroll] = useState(true);
    const consoleRef = useRef<HTMLDivElement>(null);

    // Simulate log entries
    useEffect(() => {
        const initialLogs: LogEntry[] = [
            { timestamp: new Date(Date.now() - 5000), level: 'info', message: 'Server initialized' },
            { timestamp: new Date(Date.now() - 4000), level: 'info', message: 'Loading world...' },
            { timestamp: new Date(Date.now() - 3000), level: 'warning', message: 'World load time exceeded threshold' },
            { timestamp: new Date(Date.now() - 2000), level: 'info', message: 'World loaded successfully' },
            { timestamp: new Date(Date.now() - 1000), level: 'error', message: 'Failed to bind to port 25565: Address already in use' }
        ];

        setLogs(initialLogs);

        // Simulate new log entries
        const interval = setInterval(() => {
            const levels: LogLevel[] = ['info', 'warning', 'error'];
            const level = levels[Math.floor(Math.random() * levels.length)];
            const messages = [
                'Player connected',
                'Player disconnected',
                'Server tick rate: 19.8 tps',
                'Memory usage: 1.2GB/4GB',
                'Saved world data'
            ];
            const message = messages[Math.floor(Math.random() * messages.length)];

            setLogs(prev => [...prev, { timestamp: new Date(), level, message }]);
        }, 5000);

        return () => clearInterval(interval);
    }, []);

    // Auto-scroll handling
    useEffect(() => {
        if (autoScroll && consoleRef.current) {
            consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
        }
    }, [logs, autoScroll]);

    const getLogColor = (level: LogLevel) => {
        switch (level) {
            case 'info':
                return 'text-blue-500 dark:text-blue-400';
            case 'warning':
                return 'text-yellow-500 dark:text-yellow-400';
            case 'error':
                return 'text-red-500 dark:text-red-400';
        }
    };

    const formatTimestamp = (date: Date) => {
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    };

    return (
        <div className="h-full flex flex-col bg-white dark:bg-gray-800 rounded-lg shadow">
            <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-lg font-medium">Console</h2>
                <div className="flex items-center space-x-2">
                    <label className="inline-flex items-center text-sm">
                        <input
                            type="checkbox"
                            checked={autoScroll}
                            onChange={() => setAutoScroll(!autoScroll)}
                            className="form-checkbox h-4 w-4 text-blue-500"
                        />
                        <span className="ml-1">Auto-scroll</span>
                    </label>
                    <button
                        onClick={() => setLogs([])}
                        className="text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                    >
                        Clear
                    </button>
                </div>
            </div>

            <div
                ref={consoleRef}
                className="flex-1 overflow-y-auto p-3 font-mono text-sm bg-gray-50 dark:bg-gray-900"
            >
                {logs.map((log, index) => (
                    <div key={index} className="mb-1">
                        <span className="text-gray-500 dark:text-gray-400">{formatTimestamp(log.timestamp)}</span>
                        <span className={`ml-2 ${getLogColor(log.level)}`}>[{log.level.toUpperCase()}]</span>
                        <span className="ml-2 text-gray-800 dark:text-gray-200">{log.message}</span>
                    </div>
                ))}
                {logs.length === 0 && (
                    <div className="text-center py-4 text-gray-500 dark:text-gray-400">
                        No console output
                    </div>
                )}
            </div>
        </div>
    );
};

export default Console;
