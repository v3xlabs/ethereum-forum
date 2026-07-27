import {
    infiniteQueryOptions,
    queryOptions,
    useInfiniteQuery,
    useMutation,
    useQuery,
} from '@tanstack/react-query';
import React from 'react';

import { GithubIssueComment } from '@/types/github';
import { queryClient } from '@/util/query';

import { useApi } from './api';
import { components } from './schema.gen';

// Get the Post type from schema
export type Post = components['schemas']['Post'];
export type Topic = components['schemas']['Topic'];

export const getTopics = () =>
    queryOptions({
        queryKey: ['topics'],
        queryFn: async () => {
            const response = await useApi('/topics', 'get', {});

            return response.data;
        },
    });

export const useTopicsLatest = () => useQuery(getTopics());

export const getTopicsTrending = () =>
    queryOptions({
        queryKey: ['topics', 'trending'],
        queryFn: async () => {
            const response = await useApi('/topics/trending', 'get', {});

            return response.data;
        },
    });

export const useTopicsTrending = () => useQuery(getTopicsTrending());

export const getTopic = (discourse_id: string, topicId: string) =>
    queryOptions({
        queryKey: ['topic', discourse_id, topicId],
        queryFn: async () => {
            const response = await useApi('/t/{discourse_id}/{topic_id}', 'get', {
                path: {
                    discourse_id,
                    topic_id: Number(topicId),
                },
            });

            return response.data;
        },
    });

export const useTopic = (discourse_id: string, topicId: string) =>
    useQuery(getTopic(discourse_id, topicId));

export const useTopicRefresh = (discourse_id: string, topicId: string) =>
    useMutation({
        mutationFn: async () => {
            const response = await useApi('/t/{discourse_id}/{topic_id}', 'post', {
                path: {
                    discourse_id,
                    topic_id: Number(topicId),
                },
            });

            return response.data;
        },
    });

export const getPosts = (discourse_id: string, topicId: string, page: number) =>
    queryOptions({
        queryKey: ['posts', discourse_id, topicId, page],
        queryFn: async () => {
            const response = await useApi('/t/{discourse_id}/{topic_id}/posts', 'get', {
                path: {
                    discourse_id,
                    topic_id: Number(topicId),
                },
                query: {
                    page,
                },
            });

            return response.data;
        },
    });

export const getTopicSummary = (discourse_id: string, topicId: number) =>
    queryOptions({
        queryKey: ['summary', discourse_id, topicId],
        queryFn: async () => {
            const response = await useApi('/t/{discourse_id}/{topic_id}/summary', 'get', {
                path: {
                    discourse_id,
                    topic_id: topicId,
                },
            });

            return response.data;
        },
    });

export const getPostsInfinite = (discourse_id: string, topicId: string) =>
    infiniteQueryOptions({
        queryKey: ['posts', discourse_id, topicId, 'infinite'],
        initialPageParam: 1,
        getNextPageParam: (lastPage: { posts: Post[]; has_more: boolean }, allPages) => {
            return lastPage.has_more ? allPages.length + 1 : undefined;
        },
        queryFn: async ({ pageParam }) => {
            const response = await useApi('/t/{discourse_id}/{topic_id}/posts', 'get', {
                path: {
                    discourse_id,
                    topic_id: Number(topicId),
                },
                query: {
                    page: pageParam,
                },
            });

            return response.data;
        },
    });

export const useGithubIssueComments = (issueId: number) =>
    useQuery({
        queryKey: ['githubIssues', 'ethereum/pm', issueId, 'comments'],
        queryFn: async () => {
            const response = await fetch(
                `https://api.github.com/repos/ethereum/pm/issues/${issueId}/comments`
            );
            const data = (await response.json()) as GithubIssueComment[];

            return data;
        },
    });

export const usePosts = (discourse_id: string, topicId: string, page: number) =>
    useQuery(getPosts(discourse_id, topicId, page));

export const useTopicSummary = (discourse_id: string, topicId: number) =>
    useQuery(getTopicSummary(discourse_id, topicId));

export const usePostsInfinite = (discourse_id: string, topicId: string) =>
    useInfiniteQuery(getPostsInfinite(discourse_id, topicId));

export const useStartTopicSummaryStream = () =>
    useMutation({
        mutationFn: async ({
            discourse_id,
            topicId,
        }: {
            discourse_id: string;
            topicId: number;
        }) => {
            const response = await fetch(`/api/ws/t/${discourse_id}/${topicId}/summary/stream`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
            });

            if (!response.ok) {
                throw new Error('Failed to start summary stream');
            }

            const data = await response.json();

            return data as {
                status: 'existing' | 'ongoing' | 'started';
                topic_id: number;
                summary?: string;
            };
        },
    });

// Hook for streaming topic summary
export const useTopicSummaryStream = (discourse_id: string, topicId: number) => {
    const [data, setData] = React.useState<components['schemas']['StreamingResponse'][]>([]);
    const [isLoading, setIsLoading] = React.useState(false);
    const [error, setError] = React.useState<string | null>(null);
    const [isComplete, setIsComplete] = React.useState(false);
    const [isStreaming, setIsStreaming] = React.useState(false);
    const hasReceivedDataRef = React.useRef(false);

    const startStream = React.useCallback(() => {
        if (isStreaming) return;

        // Reset state
        setData([]);
        setIsLoading(true);
        setError(null);
        setIsComplete(false);
        setIsStreaming(true);
        hasReceivedDataRef.current = false;

        const eventSource = new EventSource(`/api/ws/t/${discourse_id}/${topicId}/summary/stream`);

        eventSource.onopen = () => {
            console.log('Summary EventSource connection opened');
            setIsLoading(false);
        };

        eventSource.onmessage = (event) => {
            try {
                const response: components['schemas']['StreamingResponse'] = JSON.parse(event.data);

                hasReceivedDataRef.current = true;
                setData((prev) => [...prev, response]);

                // Check if the response indicates completion or error
                if (response.is_complete) {
                    setIsComplete(true);
                    setIsStreaming(false);
                    eventSource.close();
                } else if (response.error) {
                    setIsComplete(true);
                    setError(response.error);
                    setIsStreaming(false);
                    eventSource.close();
                }
            } catch (parseError) {
                console.error('Failed to parse summary EventSource message:', parseError);
                setError('Failed to parse server response');
                setIsComplete(true);
                setIsStreaming(false);
                eventSource.close();
            }
        };

        eventSource.onerror = (event) => {
            console.error('Summary EventSource error:', event);

            // Only show error if we haven't received any data yet
            if (!hasReceivedDataRef.current) {
                setError('Connection error occurred');
            }

            setIsLoading(false);
            setIsComplete(true);
            setIsStreaming(false);
            eventSource.close();
        };

        // Store event source for cleanup
        return () => {
            eventSource.close();
            setIsStreaming(false);
        };
    }, [topicId, isStreaming]);

    // Combine all content from the responses
    const combinedContent = React.useMemo(() => {
        return data.map((response) => response.content).join('');
    }, [data]);

    return {
        data,
        combinedContent,
        isLoading,
        error,
        isComplete,
        isStreaming,
        startStream,
    };
};

export const useDeleteTopicSummary = () =>
    useMutation({
        mutationFn: async ({
            discourse_id,
            topicId,
        }: {
            discourse_id: string;
            topicId: number;
        }) => {
            const response = await useApi('/admin/topic_summary', 'delete', {
                query: {
                    discourse_id,
                    topic_id: topicId,
                },
            });

            return response.data;
        },
        onSuccess: (_data, { discourse_id, topicId }) => {
            queryClient.invalidateQueries({
                queryKey: ['summary', discourse_id, topicId],
            });
        },
    });
