import { ClientMode, clientMode } from './server';

// Simple store for the globally accessible client mode
class ClientModeStore {
    private static instance: ClientModeStore;
    private _mode: ClientMode | null = null;
    private _initialized = false;
    private _initializing = false;
    private _initPromise: Promise<ClientMode> | null = null;

    private constructor() {}

    static getInstance(): ClientModeStore {
        if (!ClientModeStore.instance) {
            ClientModeStore.instance = new ClientModeStore();
        }
        return ClientModeStore.instance;
    }

    async getMode(): Promise<ClientMode> {
        if (this._initialized) {
            return this._mode as ClientMode;
        }

        if (!this._initializing) {
            this._initializing = true;
            this._initPromise = clientMode().then(mode => {
                this._mode = mode;
                this._initialized = true;
                return mode;
            });
        }

        return this._initPromise as Promise<ClientMode>;
    }
}

export const clientModeStore = ClientModeStore.getInstance();