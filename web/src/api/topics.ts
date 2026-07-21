import {
    infiniteQueryOptions,
    queryOptions,
    useInfiniteQuery,
    useMutation,
    useQuery,
} from '@tanstack/react-query';
import React from 'react';

import { GithubIssueComment } from '@/types/github';

import { baseUrl, useApi } from './api';
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

export const usePostsInfinite = (discourse_id: string, topicId: string) =>
    useInfiniteQuery(getPostsInfinite(discourse_id, topicId));

export type SummaryStartResponse = components['schemas']['SummaryStartResponse'];

export type SummaryStartResult =
    | { state: 'ok'; response: SummaryStartResponse }
    | { state: 'unavailable' };

export type SummaryToolCallStatus = 'running' | 'ok' | 'error';

export type SummaryToolCall = {
    call_id: string;
    tool: string;
    label: string;
    status: SummaryToolCallStatus;
    detail?: string | null;
};

export type SummaryActivity =
    | { kind: 'phase'; activityKey: string; label: string }
    | { kind: 'tool'; activityKey: string; call: SummaryToolCall };

export type SummaryStreamEvent = components['schemas']['StreamingResponse'] & {
    tool_activity?: string | null;
    is_reset?: boolean;
    tool_call?: {
        call_id: string;
        tool: string;
        label: string;
        status: string;
        detail?: string | null;
    } | null;
};

const isToolCallStatus = (status: string): status is SummaryToolCallStatus =>
    status === 'running' || status === 'ok' || status === 'error';

export type ActivityDigestTopic = {
    discourse_id: string;
    topic_id: number;
    title: string;
    slug: string;
};

// topics_included is a JSONB column typed `unknown` in the generated schema;
// the backend writes it as ActivityDigestTopic[].
export type ActivityDigest = Omit<components['schemas']['ActivityDigest'], 'topics_included'> & {
    topics_included: ActivityDigestTopic[] | null;
};

export const useStartTopicSummaryStream = () =>
    useMutation({
        mutationFn: async ({
            discourse_id,
            topicId,
            force = false,
        }: {
            discourse_id: string;
            topicId: number;
            force?: boolean;
        }): Promise<SummaryStartResult> => {
            const adminKey = localStorage.getItem('admin_key');
            const headers: Record<string, string> =
                force && adminKey ? { 'X-Admin-Key': adminKey } : {};
            const streamUrl = new URL(`t/${discourse_id}/${topicId}/summary/stream`, baseUrl);

            if (force) {
                streamUrl.searchParams.set('force', 'true');
            }

            const response = await fetch(streamUrl, {
                method: 'POST',
                headers,
            });

            if (response.status === 503) {
                return { state: 'unavailable' };
            }

            if (!response.ok) {
                throw new Error('Failed to start summary stream');
            }

            const data: SummaryStartResponse = await response.json();

            return { state: 'ok', response: data };
        },
    });

const applyToolCall = (
    activities: SummaryActivity[],
    toolCall: NonNullable<SummaryStreamEvent['tool_call']>
): SummaryActivity[] => {
    const call: SummaryToolCall = {
        call_id: toolCall.call_id,
        tool: toolCall.tool,
        label: toolCall.label,
        status: isToolCallStatus(toolCall.status) ? toolCall.status : 'running',
        detail: toolCall.detail,
    };
    const activityKey = `tool-${call.call_id}`;
    const existingIndex = activities.findIndex((a) => a.activityKey === activityKey);

    if (existingIndex === -1) {
        return [...activities, { kind: 'tool', activityKey, call }];
    }

    return activities.map((activity, index) =>
        index === existingIndex ? { kind: 'tool', activityKey, call } : activity
    );
};

// Hook for streaming topic summary
export const useTopicSummaryStream = (discourse_id: string, topicId: number) => {
    const [content, setContent] = React.useState('');
    const [activities, setActivities] = React.useState<SummaryActivity[]>([]);
    const [isLoading, setIsLoading] = React.useState(false);
    const [error, setError] = React.useState<string | null>(null);
    const [isComplete, setIsComplete] = React.useState(false);
    const [isStreaming, setIsStreaming] = React.useState(false);
    const hasReceivedDataRef = React.useRef(false);
    const phaseCounterRef = React.useRef(0);
    const eventSourceRef = React.useRef<EventSource | null>(null);

    React.useEffect(
        () => () => {
            eventSourceRef.current?.close();
        },
        []
    );

    const startStream = React.useCallback(() => {
        if (isStreaming) return;

        setContent('');
        setActivities([]);
        setIsLoading(true);
        setError(null);
        setIsComplete(false);
        setIsStreaming(true);
        hasReceivedDataRef.current = false;

        const eventSource = new EventSource(
            new URL(`t/${discourse_id}/${topicId}/summary/stream`, baseUrl)
        );

        eventSourceRef.current = eventSource;

        eventSource.onopen = () => {
            setIsLoading(false);
        };

        eventSource.onmessage = (event) => {
            try {
                const response: SummaryStreamEvent = JSON.parse(event.data);

                hasReceivedDataRef.current = true;

                if (response.is_reset) {
                    setContent('');
                }

                if (response.content) {
                    setContent((prev) => prev + response.content);
                }

                if (response.tool_activity) {
                    const label = response.tool_activity;
                    const activityKey = `phase-${phaseCounterRef.current++}`;

                    setActivities((prev) => [...prev, { kind: 'phase', activityKey, label }]);
                }

                if (response.tool_call) {
                    const toolCall = response.tool_call;

                    setActivities((prev) => applyToolCall(prev, toolCall));
                }

                if (response.is_complete) {
                    if (response.error) {
                        setError(response.error);
                    }

                    setIsComplete(true);
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

        eventSource.onerror = () => {
            if (!hasReceivedDataRef.current) {
                setError('Connection error occurred');
            }

            setIsLoading(false);
            setIsComplete(true);
            setIsStreaming(false);
            eventSource.close();
        };
    }, [discourse_id, topicId, isStreaming]);

    return {
        combinedContent: content,
        activities,
        isLoading,
        error,
        isComplete,
        isStreaming,
        startStream,
    };
};

export type CachedTopicSummary = {
    summary_text: string;
    based_on: string;
    based_on_post_number?: number | null;
    created_at?: string;
};

export const getCachedTopicSummary = (discourse_id: string, topicId: number) =>
    queryOptions({
        queryKey: ['topic-summary-cached', discourse_id, topicId],
        queryFn: async (): Promise<CachedTopicSummary | null> => {
            const response = await fetch(
                new URL(`t/${discourse_id}/${topicId}/summary/cached`, baseUrl)
            );

            if (response.status === 404) {
                return null;
            }

            if (!response.ok) {
                throw new Error('Failed to fetch cached summary');
            }

            const data: CachedTopicSummary = await response.json();

            return data;
        },
        staleTime: 60 * 1000,
        retry: 1,
    });

export const useCachedTopicSummary = (discourse_id: string, topicId: number) =>
    useQuery(getCachedTopicSummary(discourse_id, topicId));

export const getActivityDigest = () =>
    queryOptions({
        queryKey: ['digest'],
        queryFn: async (): Promise<ActivityDigest | null> => {
            const response = await fetch(new URL('digest', baseUrl));

            if (response.status === 404) {
                return null;
            }

            if (!response.ok) {
                throw new Error('Failed to fetch activity digest');
            }

            const data: ActivityDigest = await response.json();

            return data;
        },
        staleTime: 5 * 60 * 1000,
        retry: 1,
    });

export const useActivityDigest = () => useQuery(getActivityDigest());
