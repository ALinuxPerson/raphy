import React from 'react';

interface NavigationButtonsProps {
    onConnect: () => void;
    onProceed: () => void;
    canConnect: boolean;
    canProceed: boolean;
}

const NavigationButtons: React.FC<NavigationButtonsProps> = ({
                                                                 onConnect,
                                                                 onProceed,
                                                                 canConnect,
                                                                 canProceed
                                                             }) => {
    return (
        <div className="flex items-center justify-between w-full max-w-md mx-auto">
            {/* Left Arrow */}
            <button
                className="w-10 h-10 rounded-full border border-gray-300 flex items-center justify-center text-gray-500 dark:border-gray-600 dark:text-gray-400"
                disabled={true} // Always disabled as per design
            >
                <svg className="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M15 18l-6-6 6-6" />
                </svg>
            </button>

            {/* Connect Button */}
            <button
                onClick={onConnect}
                disabled={!canConnect}
                className={`px-6 py-2 rounded-md text-white transition-all duration-200 ${
                    canConnect
                        ? 'bg-blue-500 hover:bg-blue-600 active:bg-blue-700'
                        : 'bg-gray-400 cursor-not-allowed opacity-60'
                }`}
            >
                Connect
            </button>

            {/* Right Arrow */}
            <button
                onClick={onProceed}
                disabled={!canProceed}
                className={`w-10 h-10 rounded-full border flex items-center justify-center transition-all duration-200 ${
                    canProceed
                        ? 'border-blue-500 text-blue-500 dark:border-blue-400 dark:text-blue-400 animate-pulse hover:bg-blue-50 dark:hover:bg-blue-900/20'
                        : 'border-gray-300 text-gray-500 dark:border-gray-600 dark:text-gray-400'
                }`}
            >
                <svg className="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M9 18l6-6-6-6" />
                </svg>
            </button>
        </div>
    );
};

export default NavigationButtons;