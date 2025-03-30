import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

type LogSource = 'stdout' | 'stderr' | 'input';

interface LogEntry {
    source: LogSource;
    message: string;
    timestamp: Date;
}

const Console: React.FC = () => {
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [autoScroll, setAutoScroll] = useState(true);
    const [inputValue, setInputValue] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const consoleRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);

    // Listen for stdout and stderr events from Tauri
    useEffect(() => {
        const stdoutListener = listen<string>('stdout', (event) => {
            const message = event.payload;
            setLogs(prev => [...prev, {
                timestamp: new Date(),
                source: 'stdout',
                message
            }]);
        });

        const stderrListener = listen<string>('stderr', (event) => {
            const message = event.payload;
            setLogs(prev => [...prev, {
                timestamp: new Date(),
                source: 'stderr',
                message
            }]);
        });

        return () => {
            stdoutListener.then(unlistenFn => unlistenFn());
            stderrListener.then(unlistenFn => unlistenFn());
        };
    }, []);

    // Auto-scroll handling
    useEffect(() => {
        if (autoScroll && consoleRef.current) {
            consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
        }
    }, [logs, autoScroll]);

    // Focus on the input field when component mounts
    useEffect(() => {
        if (inputRef.current) {
            inputRef.current.focus();
        }
    }, []);

    const getLogColor = (source: LogSource) => {
        switch (source) {
            case 'stdout':
                return 'text-gray-800 dark:text-gray-200';
            case 'stderr':
                return 'text-red-500 dark:text-red-400';
            case 'input':
                return 'text-blue-500 dark:text-blue-400 font-semibold';
        }
    };

    const handleInputSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        
        if (!inputValue.trim()) return;
        
        // Add input to logs with special formatting
        setLogs(prev => [...prev, {
            timestamp: new Date(),
            source: 'input',
            message: `> ${inputValue}`
        }]);
        
        try {
            setIsSubmitting(true);
            
            // Convert input to UTF-8 bytes
            const encoder = new TextEncoder();
            const data = encoder.encode(inputValue + '\n');
            
            // Send to server using the stdin protocol
            const response = await fetch(`stdin://localhost`, {
                method: 'POST',
                body: data
            });
            
            if (!response.ok) {
                throw new Error(`Failed to send input: ${response.status}`);
            }
        } catch (error) {
            console.error('Error sending input:', error);
            
            // Add error to logs
            setLogs(prev => [...prev, {
                timestamp: new Date(),
                source: 'stderr',
                message: `Failed to send command: ${error}`
            }]);
        } finally {
            setIsSubmitting(false);
            setInputValue('');
            
            // Refocus the input field
            if (inputRef.current) {
                inputRef.current.focus();
            }
        }
    };

    return (
        <div className="h-120 flex flex-col bg-white dark:bg-gray-800 rounded-lg shadow">
            <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-lg font-medium">Console</h2>
                <div className="flex items-center space-x-2">
                    <label className="inline-flex items-center text-sm">
                        <input
                            type="checkbox"
                            checked={autoScroll}
                            onChange={() => setAutoScroll(!autoScroll)}
                            className="form-checkbox h-4 w-4 text-blue-500 rounded"
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
                className="flex-1 overflow-y-auto p-3 font-mono text-sm bg-gray-50 dark:bg-gray-900 h-64"
            >
                {logs.map((log, index) => (
                    <div key={index} className="mb-1">
                        <span className={`${getLogColor(log.source)}`}>{log.message}</span>
                    </div>
                ))}
                {logs.length === 0 && (
                    <div className="text-center py-4 text-gray-500 dark:text-gray-400">
                        No console output
                    </div>
                )}
            </div>
            
            {/* Command input area */}
            <form 
                onSubmit={handleInputSubmit}
                className="border-t border-gray-200 dark:border-gray-700 p-3 flex items-center"
            >
                <div className="flex-1 relative">
                    <input
                        ref={inputRef}
                        type="text"
                        value={inputValue}
                        onChange={(e) => setInputValue(e.target.value)}
                        placeholder="Type a command and press Enter..."
                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                                 bg-white dark:bg-gray-800 font-mono text-sm focus:outline-none 
                                 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        disabled={isSubmitting}
                    />
                    {isSubmitting && (
                        <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
                            <svg className="animate-spin h-4 w-4 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                            </svg>
                        </div>
                    )}
                </div>
                <button
                    type="submit"
                    disabled={isSubmitting || !inputValue.trim()}
                    className={`ml-2 px-4 py-2 rounded-md text-sm ${
                        isSubmitting || !inputValue.trim()
                            ? 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 cursor-not-allowed'
                            : 'bg-blue-500 hover:bg-blue-600 text-white transition-colors'
                    }`}
                >
                    Send
                </button>
            </form>
        </div>
    );
};

export default Console;