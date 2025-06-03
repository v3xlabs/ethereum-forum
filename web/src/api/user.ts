import { useQuery } from '@tanstack/react-query';

import { useApi } from './api';

const getUser = (username: string) => {
    return {
        queryKey: ['user', username],
        queryFn: async () => {
            const response = await useApi('/user/{username}', 'get', {
                path: {
                    username: username,
                },
            });

            return response.data;
        },
    };
};

export const useUser = (username: string) => useQuery(getUser(username));
