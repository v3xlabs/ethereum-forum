import { ReactNode } from 'react';
import { FiLock, FiUser } from 'react-icons/fi';

import { getSSOLoginUrl, useAuth, useSSOProviders } from '../api/auth';

interface AuthGuardProps {
    children: ReactNode;
    fallback?: ReactNode;
    requireAuth?: boolean;
}

// Standard auth guard for general features
export const AuthGuard = ({ children, fallback, requireAuth = true }: AuthGuardProps) => {
    const { isAuthenticated, isLoading } = useAuth();

    if (!requireAuth || isAuthenticated) {
        return <>{children}</>;
    }

    if (isLoading) {
        return (
            <div className="flex items-center justify-center p-8">
                <div className="flex items-center gap-2 text-primary">
                    <FiUser size={16} className="animate-pulse" />
                    <span>Loading...</span>
                </div>
            </div>
        );
    }

    return <>{fallback || <AuthRequiredFallback />}</>;
};

const AuthRequiredFallback = () => {
    const { data: providersData, isLoading, error } = useSSOProviders();

    const handleProviderClick = async (providerId: string) => {
        try {
            const response = await getSSOLoginUrl(providerId);

            window.location.href = response.redirect_url;
        } catch (error) {
            console.error('Failed to get login URL:', error);
        }
    };

    return (
        <div className="flex flex-col items-center justify-center p-8 space-y-6 text-center">
            <div className="flex items-center gap-3 text-xl font-semibold text-primary">
                <FiLock size={24} />
                <span>Authentication Required</span>
            </div>

            <p className="text-primary max-w-md">
                This feature requires you to be signed in. Please choose a sign-in method below to
                continue.
            </p>

            {isLoading ? (
                <div className="flex items-center gap-2 text-primary">
                    <FiUser size={16} className="animate-pulse" />
                    <span>Loading sign-in options...</span>
                </div>
            ) : error || !providersData?.providers?.length ? (
                <div className="text-primary">
                    <p>Sign-in is currently unavailable. Please try again later.</p>
                </div>
            ) : (
                <div className="space-y-4">
                    <p className="text-sm text-primary font-medium">Choose a sign-in method:</p>
                    <div className="flex flex-col gap-2 min-w-[200px]">
                        {providersData.providers.map((providerId) => (
                            <button
                                key={providerId}
                                onClick={() => handleProviderClick(providerId)}
                                className="flex items-center gap-2 px-4 py-3 bg-secondary text-primary border border-primary rounded-md hover:bg-tertiary transition-colors"
                            >
                                <FiUser size={16} />
                                <span className="capitalize">Sign in with {providerId}</span>
                            </button>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
};
