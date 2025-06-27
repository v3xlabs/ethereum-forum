import { createFetch } from 'openapi-hooks';
import { toast } from 'sonner';

import { paths } from './schema.gen';

export const baseUrl = new URL('/api/', import.meta.env.VITE_API_URL ?? window.location.origin);

export const useApi = createFetch<paths>({
    baseUrl,
    async headers() {
        const token = localStorage.getItem('auth_token');
        const adminKey = localStorage.getItem('forum.ethereum.adminKey');

        return {
            ...(token && { Authorization: `Bearer ${token}` }),
            ...(adminKey && { 'X-Admin-Key': adminKey }),
        };
    },
    onError(error: { status: number }) {
        // If we get a 401, clear auth data
        if (error.status === 401) {
            localStorage.removeItem('auth_token');
            localStorage.removeItem('auth_user');
            localStorage.removeItem('auth_expires_at');

            // Let the auth system handle the state update
            window.dispatchEvent(new Event('auth-cleared'));
        }

        if (error.status === 429) {
            toast.error('Request throttled, please wait a moment before retrying');
        }
    },
});

export const queryOptions = () => {};
