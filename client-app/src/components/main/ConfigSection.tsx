import React, {useEffect, useState} from 'react';
import {
    ArgumentsKind,
    ClientMode,
    ConfigMask,
    getServerConfig,
    isManualArguments,
    isParsedArguments,
    JavaPathKind,
    ResolvedConfig,
    updateConfig,
    UserKind
} from "../../utils/server.ts";
import {listen} from "@tauri-apps/api/event";

// Component Props
interface ConfigSectionProps {
    clientMode: ClientMode,
    isConfigMissing: boolean,
    setIsConfigMissing: (value: (((prevState: boolean) => boolean) | boolean)) => void
}

// Config State Interface
interface ConfigState {
    javaPath: string;
    serverPath: string;
    parsedJavaArguments: string;
    manualJavaArguments: string[];
    javaArgumentsKind: ArgumentsKind;
    parsedServerArguments: string;
    manualServerArguments: string[];
    serverArgumentsKind: ArgumentsKind;
    user: string | null;
    javaPathMask: JavaPathKind;
    userMask: UserKind;
    configChanged: boolean;
}

// Toggle Button Props
interface ToggleButtonProps {
    active: boolean;
    onClick: () => void;
    children: React.ReactNode;
}

// Text Input Props
interface TextInputProps {
    value: string;
    onChange: (value: string) => void;
    placeholder: string;
    showBrowseButton?: boolean;
}

// Toggle Button Component
const ToggleButton: React.FC<ToggleButtonProps> = ({active, onClick, children}) => (
    <button
        onClick={onClick}
        className={`px-3 py-1 text-sm ${
            active
                ? 'bg-blue-500 text-white'
                : 'bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700'
        }`}
    >
        {children}
    </button>
);

// Toggle Button Group
const ToggleButtonGroup: React.FC<{
    label: string;
    children: React.ReactNode;
}> = ({label, children}) => (
    <div className="flex items-center mb-2">
        <label className="block text-sm font-medium">{label}</label>
        <div className="ml-4 inline-flex rounded-md overflow-hidden border border-gray-300 dark:border-gray-700">
            {children}
        </div>
    </div>
);

// Text Input with optional browse button
const TextInput: React.FC<TextInputProps> = ({
                                                 value,
                                                 onChange,
                                                 placeholder,
                                                 showBrowseButton = false,
                                             }) => (
    <div className="flex">
        <input
            type="text"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white
                      dark:bg-gray-800 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder={placeholder}
        />
        {showBrowseButton && (
            <button
                className="ml-2 px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-md text-sm hover:bg-gray-200
                           dark:hover:bg-gray-600 transition-colors"
            >
                Browse...
            </button>
        )}
    </div>
);

// Manual Arguments Component
const ManualArgumentsList: React.FC<{
    arguments: string[];
    onUpdate: (index: number, value: string) => void;
    onRemove: (index: number) => void;
    onAdd: () => void;
}> = ({arguments: args, onUpdate, onRemove, onAdd}) => (
    <div className="space-y-2">
        {args.map((arg, index) => (
            <div key={index} className="flex items-center">
                <input
                    type="text"
                    value={arg}
                    onChange={(e) => onUpdate(index, e.target.value)}
                    className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md
                              bg-white dark:bg-gray-800 text-sm font-mono focus:outline-none
                              focus:ring-2 focus:ring-blue-500"
                    placeholder="Argument"
                />
                <button
                    onClick={() => onRemove(index)}
                    className="ml-2 p-2 text-gray-500 dark:text-gray-400 hover:text-red-500
                             dark:hover:text-red-400 transition-colors"
                >
                    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
        ))}
        <button
            onClick={onAdd}
            className="flex items-center text-blue-500 hover:text-blue-600 text-sm"
        >
            <svg className="w-4 h-4 mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4"/>
            </svg>
            Add Argument
        </button>
    </div>
);

// Action Buttons Component
const ActionButtons: React.FC<{
    configChanged: boolean;
    onReset: () => void;
    onSave: () => Promise<void>;
}> = ({configChanged, onReset, onSave}) => (
    <div className="flex justify-end space-x-3 pt-2">
        {configChanged && (
            <button
                onClick={onReset}
                className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm
                         hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
                Reset
            </button>
        )}
        <button
            onClick={onSave}
            disabled={!configChanged}
            className={`px-4 py-2 rounded-md text-sm transition-colors ${
                configChanged
                    ? 'bg-blue-500 hover:bg-blue-600 text-white'
                    : 'bg-gray-300 dark:bg-gray-700 text-gray-500 dark:text-gray-400 cursor-not-allowed'
            }`}
        >
            Save
        </button>
    </div>
);

// Main ConfigSection Component
const ConfigSection: React.FC<ConfigSectionProps> = ({clientMode, isConfigMissing, setIsConfigMissing}) => {
    // State initialization with default values
    const [config, setConfig] = useState<ConfigState>({
        javaPath: '',
        serverPath: '',
        parsedJavaArguments: '',
        manualJavaArguments: [],
        javaArgumentsKind: ArgumentsKind.Parsed,
        parsedServerArguments: '',
        manualServerArguments: [],
        serverArgumentsKind: ArgumentsKind.Parsed,
        user: null,
        javaPathMask: JavaPathKind.AutoDetect,
        userMask: UserKind.Current,
        configChanged: false
    });

    // Add this to track the original config
    const [originalConfig, setOriginalConfig] = useState<ConfigState | null>(null);

    // Helper function to update config state and mark as changed
    const updateComponentConfig = (updates: Partial<ConfigState>) => {
        setConfig(prev => {
            const updated = {...prev, ...updates};

            // Compare with original config to determine if anything has changed
            if (originalConfig) {
                const isChanged =
                    updated.javaPath !== originalConfig.javaPath ||
                    updated.serverPath !== originalConfig.serverPath ||
                    updated.parsedJavaArguments !== originalConfig.parsedJavaArguments ||
                    updated.manualJavaArguments.length !== originalConfig.manualJavaArguments.length ||
                    updated.manualJavaArguments.some((arg, i) => arg !== originalConfig.manualJavaArguments[i]) ||
                    updated.javaArgumentsKind !== originalConfig.javaArgumentsKind ||
                    updated.parsedServerArguments !== originalConfig.parsedServerArguments ||
                    updated.manualServerArguments.length !== originalConfig.manualServerArguments.length ||
                    updated.manualServerArguments.some((arg, i) => arg !== originalConfig.manualServerArguments[i]) ||
                    updated.serverArgumentsKind !== originalConfig.serverArgumentsKind ||
                    updated.user !== originalConfig.user ||
                    updated.javaPathMask !== originalConfig.javaPathMask ||
                    updated.userMask !== originalConfig.userMask;

                return {...updated, configChanged: isChanged};
            }

            return {...updated, configChanged: true};
        });
    };

    // Helper function to process server config data
    const processConfigData = (
        resolvedConfig: ResolvedConfig,
        configMask: ConfigMask,
        isChanged: boolean = false
    ): ConfigState => {
        // Extract java arguments
        let javaArgs = {
            manualJavaArguments: [] as string[],
            parsedJavaArguments: '',
            javaArgumentsKind: ArgumentsKind.Parsed
        };

        if (isManualArguments(resolvedConfig.java_arguments)) {
            javaArgs = {
                ...javaArgs,
                manualJavaArguments: resolvedConfig.java_arguments.Manual,
                javaArgumentsKind: ArgumentsKind.Manual
            };
        }

        if (isParsedArguments(resolvedConfig.java_arguments)) {
            javaArgs = {
                ...javaArgs,
                parsedJavaArguments: resolvedConfig.java_arguments.Parsed,
                javaArgumentsKind: ArgumentsKind.Parsed
            };
        }

        // Extract server arguments
        let serverArgs = {
            manualServerArguments: [] as string[],
            parsedServerArguments: '',
            serverArgumentsKind: ArgumentsKind.Parsed
        };

        if (isManualArguments(resolvedConfig.server_arguments)) {
            serverArgs = {
                ...serverArgs,
                manualServerArguments: resolvedConfig.server_arguments.Manual,
                serverArgumentsKind: ArgumentsKind.Manual
            };
        }

        if (isParsedArguments(resolvedConfig.server_arguments)) {
            serverArgs = {
                ...serverArgs,
                parsedServerArguments: resolvedConfig.server_arguments.Parsed,
                serverArgumentsKind: ArgumentsKind.Parsed
            };
        }

        // Return the complete config state
        return {
            javaPath: resolvedConfig.java_path,
            serverPath: resolvedConfig.server_jar_path,
            ...serverArgs,
            ...javaArgs,
            user: resolvedConfig.user,
            javaPathMask: configMask.java_path,
            userMask: configMask.user,
            configChanged: isChanged
        };
    };


    // Modify the loadConfig function in the first useEffect
    useEffect(() => {
        const loadConfig = async () => {
            try {
                const configData = await getServerConfig();
                if (configData) {
                    const [resolvedConfig, configMask] = configData;
                    const processedConfig = processConfigData(resolvedConfig, configMask);
                    setConfig(processedConfig);
                    setOriginalConfig(processedConfig); // Store the original config
                    setIsConfigMissing(false);
                } else {
                    setIsConfigMissing(true);
                }
            } catch (error) {
                console.error('Failed to load server config:', error);
            }
        };

        loadConfig();
    }, []);

    // Setup event listener for config updates
    useEffect(() => {
        const unlisten = listen<[ResolvedConfig, ConfigMask]>("config-updated", (event) => {
            const [resolvedConfig, configMask] = event.payload;
            const processedConfig = processConfigData(resolvedConfig, configMask);
            setConfig(processedConfig);
            setOriginalConfig(processedConfig); // Update original config on external changes
        });

        return () => {
            unlisten.then(unlistenFn => unlistenFn());
        };
    }, []);

    // Reset handler
    const handleReset = () => {
        if (originalConfig) {
            setConfig({...originalConfig, configChanged: false});
        }
    };

    // Save handler
    const handleSave = async () => {
        const resolvedConfig: ResolvedConfig = {
            java_path: config.javaPath,
            server_jar_path: config.serverPath,
            java_arguments: config.javaArgumentsKind === ArgumentsKind.Parsed
                ? {Parsed: config.parsedJavaArguments}
                : {Manual: config.manualJavaArguments},
            server_arguments: config.serverArgumentsKind === ArgumentsKind.Parsed
                ? {Parsed: config.parsedServerArguments}
                : {Manual: config.manualServerArguments},
            user: config.user
        };

        const configMask: ConfigMask = {
            java_path: config.javaPathMask,
            arguments: config.serverArgumentsKind,
            user: config.userMask
        };

        try {
            await updateConfig(resolvedConfig, configMask);
            setConfig(prev => ({...prev, configChanged: false}));
            setOriginalConfig({...config, configChanged: false}); // Update original config after save
        } catch (error) {
            console.error('Failed to save server config:', error);
        }

    };

    // Manual java arguments handlers
    const addJavaArgument = () => {
        updateComponentConfig({
            manualJavaArguments: [...config.manualServerArguments, '']
        });
    };

    const updateJavaArgument = (index: number, value: string) => {
        const newArgs = [...config.manualJavaArguments];
        newArgs[index] = value;
        updateComponentConfig({manualJavaArguments: newArgs});
    };

    const removeJavaArgument = (index: number) => {
        const newArgs = [...config.manualJavaArguments];
        newArgs.splice(index, 1);
        updateComponentConfig({manualJavaArguments: newArgs});
    };

    // Manual server arguments handlers
    const addServerArgument = () => {
        updateComponentConfig({
            manualServerArguments: [...config.manualServerArguments, '']
        });
    };

    const updateServerArgument = (index: number, value: string) => {
        const newArgs = [...config.manualServerArguments];
        newArgs[index] = value;
        updateComponentConfig({manualServerArguments: newArgs});
    };

    const removeServerArgument = (index: number) => {
        const newArgs = [...config.manualServerArguments];
        newArgs.splice(index, 1);
        updateComponentConfig({manualServerArguments: newArgs});
    };

    // If config is missing, show a notification
    if (isConfigMissing) {
        return (
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6 transition-all">
                <div className="text-center py-8">
                    <svg className="w-16 h-16 mx-auto text-gray-400 dark:text-gray-500 mb-4" fill="none"
                         viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5}
                              d="M9.663 17h4.673M12 3v1m0 16v1m-9-9h1m16 0h1m-2.947-7.053l-.708.708M5.655 5.655l-.708.708m0 11.314l.708-.708m11.314 0l.708.708M8 12h.01M12 12h.01M16 12h.01M9 12a3 3 0 11-6 0 3 3 0 016 0zm6 0a3 3 0 11-6 0 3 3 0 016 0zm6 0a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    <h2 className="text-xl font-medium mb-3">Configuration Missing</h2>
                    <p className="text-gray-600 dark:text-gray-400 mb-6 max-w-md mx-auto">
                        The server configuration needs to be set up before you can proceed. Please complete the
                        configuration using the form below.
                    </p>
                    <button
                        onClick={() => setIsConfigMissing(false)}
                        className="px-4 py-2 rounded-md bg-blue-500 hover:bg-blue-600 text-white transition-colors"
                    >
                        Set Up Configuration
                    </button>
                </div>
            </div>
        );
    }

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6 transition-all">
            <h2 className="text-lg font-medium mb-4">Configuration</h2>

            <div className="space-y-4">
                {/* Java Path */}
                <div>
                    <ToggleButtonGroup label="Java Path">
                        <ToggleButton
                            active={config.javaPathMask === JavaPathKind.AutoDetect}
                            onClick={() => updateComponentConfig({javaPathMask: JavaPathKind.AutoDetect})}
                        >
                            Auto-detect
                        </ToggleButton>
                        <ToggleButton
                            active={config.javaPathMask === JavaPathKind.Custom}
                            onClick={() => updateComponentConfig({javaPathMask: JavaPathKind.Custom})}
                        >
                            Custom
                        </ToggleButton>
                    </ToggleButtonGroup>

                    {config.javaPathMask === JavaPathKind.Custom && (
                        <TextInput
                            value={config.javaPath}
                            onChange={(value) => updateComponentConfig({javaPath: value})}
                            placeholder="/usr/bin/java"
                            showBrowseButton={clientMode === ClientMode.Local}
                        />
                    )}
                </div>

                {/* Server Path */}
                <div>
                    <label className="block text-sm font-medium mb-2">Server Path</label>
                    <TextInput
                        value={config.serverPath}
                        onChange={(value) => updateComponentConfig({serverPath: value})}
                        placeholder="/User/alp/doobiee/Public/paper/server.jar"
                        showBrowseButton={clientMode === ClientMode.Local}
                    />
                </div>

                {/* Java Arguments */}
                <div>
                    <ToggleButtonGroup label="Java Arguments">
                        <ToggleButton
                            active={config.javaArgumentsKind === ArgumentsKind.Parsed}
                            onClick={() => updateComponentConfig({javaArgumentsKind: ArgumentsKind.Parsed})}
                        >
                            Parsed
                        </ToggleButton>
                        <ToggleButton
                            active={config.javaArgumentsKind === ArgumentsKind.Manual}
                            onClick={() => updateComponentConfig({javaArgumentsKind: ArgumentsKind.Manual})}
                        >
                            Manual
                        </ToggleButton>
                    </ToggleButtonGroup>

                    {config.javaArgumentsKind === ArgumentsKind.Parsed ? (
                        <TextInput
                            value={config.parsedJavaArguments}
                            onChange={(value) => updateComponentConfig({parsedJavaArguments: value})}
                            placeholder="-Xmx4096M -Xms4096M"
                        />
                    ) : (
                        <ManualArgumentsList
                            arguments={config.manualJavaArguments}
                            onUpdate={updateJavaArgument}
                            onRemove={removeJavaArgument}
                            onAdd={addJavaArgument}
                        />
                    )}
                </div>

                {/* Server Arguments */}
                <div>
                    <ToggleButtonGroup label="Server Arguments">
                        <ToggleButton
                            active={config.serverArgumentsKind === ArgumentsKind.Parsed}
                            onClick={() => updateComponentConfig({serverArgumentsKind: ArgumentsKind.Parsed})}
                        >
                            Parsed
                        </ToggleButton>
                        <ToggleButton
                            active={config.serverArgumentsKind === ArgumentsKind.Manual}
                            onClick={() => updateComponentConfig({serverArgumentsKind: ArgumentsKind.Manual})}
                        >
                            Manual
                        </ToggleButton>
                    </ToggleButtonGroup>

                    {config.serverArgumentsKind === ArgumentsKind.Parsed ? (
                        <TextInput
                            value={config.parsedServerArguments}
                            onChange={(value) => updateComponentConfig({parsedServerArguments: value})}
                            placeholder="nogui"
                        />
                    ) : (
                        <ManualArgumentsList
                            arguments={config.manualServerArguments}
                            onUpdate={updateServerArgument}
                            onRemove={removeServerArgument}
                            onAdd={addServerArgument}
                        />
                    )}
                </div>

                {/* User */}
                <div>
                    <ToggleButtonGroup label="User">
                        <ToggleButton
                            active={config.userMask === UserKind.Current}
                            onClick={() => updateComponentConfig({userMask: UserKind.Current})}
                        >
                            Current
                        </ToggleButton>
                        <ToggleButton
                            active={config.userMask === UserKind.Specific}
                            onClick={() => updateComponentConfig({userMask: UserKind.Specific})}
                        >
                            Specific
                        </ToggleButton>
                    </ToggleButtonGroup>

                    {config.userMask === UserKind.Specific && (
                        <TextInput
                            value={config.user || ''}
                            onChange={(value) => updateComponentConfig({user: value})}
                            placeholder="doobiee"
                        />
                    )}
                </div>

                {/* Action Buttons */}
                <ActionButtons
                    configChanged={config.configChanged}
                    onReset={handleReset}
                    onSave={handleSave}
                />
            </div>
        </div>
    );
};

export default ConfigSection;