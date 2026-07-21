import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';

import { useApi } from './api';
import type { components } from './schema.gen';

// Export types from the generated schema
export type SSOProvidersResponse = components['schemas']['SSOProvidersResponse'];
export type LoginResponse = components['schemas']['LoginResponse'];
export type AuthResponse = components['schemas']['AuthResponse'];
export type TokenValidationResponse = components['schemas']['TokenValidationResponse'];
export type User = components['schemas']['User'];
export type UserInfo = components['schemas']['UserInfo'];

// Auth state type
export interface AuthState {
    user: User | null;
    token: string | null;
    isAuthenticated: boolean;
}

// Helper function to convert UserInfo to User type
const mapUserInfoToUser = (userInfo: UserInfo): User => {
    return {
        user_id: userInfo.sub,
        username: userInfo.name,
        display_name: userInfo.name,
        email: userInfo.email,
        avatar_url: undefined,
        sso_provider: 'unknown',
        sso_user_id: userInfo.sub,
        extras: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        last_login_at: new Date().toISOString(),
    };
};

// Get authentication state
const getAuthState = () => {
    return {
        queryKey: ['auth', 'state'],
        queryFn: async (): Promise<AuthState> => {
            const token = localStorage.getItem('auth_token');
            const userStr = localStorage.getItem('auth_user');
            const expiresAt = localStorage.getItem('auth_expires_at');

            if (!token || !userStr || !expiresAt) {
                return {
                    user: null,
                    token: null,
                    isAuthenticated: false,
                };
            }

            // Check if token is expired
            const expirationTime = parseInt(expiresAt, 10);
            const now = Math.floor(Date.now() / 1000);

            if (expirationTime <= now) {
                // Token expired, clear data
                localStorage.removeItem('auth_token');
                localStorage.removeItem('auth_user');
                localStorage.removeItem('auth_expires_at');

                return {
                    user: null,
                    token: null,
                    isAuthenticated: false,
                };
            }

            try {
                // Validate token with server
                const response = await useApi('/user/token/validate', 'post', {
                    query: { token },
                });

                if (response.data.valid && response.data.user) {
                    const user = mapUserInfoToUser(response.data.user);

                    // Update localStorage with fresh user data
                    localStorage.setItem('auth_user', JSON.stringify(user));

                    return {
                        user,
                        token,
                        isAuthenticated: true,
                    };
                } else {
                    // Token is invalid, clear data
                    localStorage.removeItem('auth_token');
                    localStorage.removeItem('auth_user');
                    localStorage.removeItem('auth_expires_at');

                    return {
                        user: null,
                        token: null,
                        isAuthenticated: false,
                    };
                }
            } catch (error) {
                console.error('Token validation failed:', error);

                // On validation error, clear auth data
                localStorage.removeItem('auth_token');
                localStorage.removeItem('auth_user');
                localStorage.removeItem('auth_expires_at');

                return {
                    user: null,
                    token: null,
                    isAuthenticated: false,
                };
            }
        },
        staleTime: 5 * 60 * 1000, // Consider fresh for 5 minutes
        refetchOnWindowFocus: true,
        retry: false, // Don't retry auth failures
    };
};

// Get available SSO providers
const getSSOProviders = () => {
    return {
        queryKey: ['sso', 'providers'],
        queryFn: async (): Promise<SSOProvidersResponse> => {
            const response = await useApi('/user/sso/providers', 'get', {});

            return response.data;
        },
    };
};

// Hooks
export const useAuth = () => {
    const query = useQuery(getAuthState());
    const queryClient = useQueryClient();

    // Listen for auth-cleared events from the API middleware
    useEffect(() => {
        const handleAuthCleared = () => {
            queryClient.setQueryData(['auth', 'state'], {
                user: null,
                token: null,
                isAuthenticated: false,
            });
        };

        window.addEventListener('auth-cleared', handleAuthCleared);

        return () => window.removeEventListener('auth-cleared', handleAuthCleared);
    }, [queryClient]);

    return {
        user: query.data?.user || null,
        token: query.data?.token || null,
        isAuthenticated: query.data?.isAuthenticated || false,
        isLoading: query.isLoading,
        isValidating: query.isFetching && !query.isLoading,
        error: query.error,
        refetch: query.refetch,
    };
};

export const useSSOProviders = () => useQuery(getSSOProviders());

// Get SSO login URL
export const getSSOLoginUrl = async (providerId: string): Promise<LoginResponse> => {
    const response = await useApi('/user/sso/{sso_id}/login', 'get', {
        path: {
            sso_id: providerId,
        },
    });

    return response.data;
};

// Handle SSO callback
export const handleSSOCallback = async (
    providerId: string,
    code: string,
    state?: string
): Promise<AuthResponse> => {
    const response = await useApi('/user/sso/{sso_id}/callback', 'get', {
        path: {
            sso_id: providerId,
        },
        query: {
            code,
            _state_param: state,
        },
    });

    return response.data;
};

// Validate JWT token
export const validateToken = async (token: string): Promise<TokenValidationResponse> => {
    const response = await useApi('/user/token/validate', 'post', {
        query: {
            token,
        },
    });

    return response.data;
};

// Login mutation
export const useLogin = () => {
    const queryClient = useQueryClient();

    return useMutation({
        mutationFn: async ({
            token,
            user,
            expiresAt,
        }: {
            token: string;
            user: User;
            expiresAt: number;
        }) => {
            // Store in localStorage
            localStorage.setItem('auth_token', token);
            localStorage.setItem('auth_user', JSON.stringify(user));
            localStorage.setItem('auth_expires_at', expiresAt.toString());

            return { token, user, expiresAt };
        },
        onSuccess: (data) => {
            // Immediately update the auth query cache
            queryClient.setQueryData(['auth', 'state'], {
                user: data.user,
                token: data.token,
                isAuthenticated: true,
            });
        },
    });
};

// Logout mutation
export const useLogout = () => {
    const queryClient = useQueryClient();

    return useMutation({
        mutationFn: async () => {
            // Clear localStorage
            localStorage.removeItem('auth_token');
            localStorage.removeItem('auth_user');
            localStorage.removeItem('auth_expires_at');
        },
        onSuccess: () => {
            // Immediately update the auth query cache
            queryClient.setQueryData(['auth', 'state'], {
                user: null,
                token: null,
                isAuthenticated: false,
            });

            // Clear all queries that might depend on authentication
            queryClient.clear(); // Clear entire cache to be safe
        },
    });
};
