import {
    infiniteQueryOptions,
    queryOptions,
    useInfiniteQuery,
    useMutation,
    useQuery,
} from '@tanstack/react-query';

import { GithubIssueComment } from '@/types/github';

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

export const getTopic = (topicId: string) =>
    queryOptions({
        queryKey: ['topic', topicId],
        queryFn: async () => {
            const response = await useApi('/t/{topic_id}', 'get', {
                path: {
                    topic_id: Number(topicId),
                },
            });

            return response.data;
        },
    });

export const useTopic = (topicId: string) => useQuery(getTopic(topicId));

export const useTopicRefresh = (topicId: string) =>
    useMutation({
        mutationFn: async () => {
            const response = await useApi('/t/{topic_id}', 'post', {
                path: {
                    topic_id: Number(topicId),
                },
            });

            return response.data;
        },
    });

export const getPosts = (topicId: string, page: number) =>
    queryOptions({
        queryKey: ['posts', topicId, page],
        queryFn: async () => {
            const response = await useApi('/t/{topic_id}/posts', 'get', {
                path: {
                    topic_id: Number(topicId),
                },
                query: {
                    page,
                },
            });

            return response.data;
        },
    });

export const getPost = (postId: string) =>
    queryOptions({
        queryKey: ['individualPost', postId],
        queryFn: async () => {
            const response = await useApi('/post/{post_id}', 'get', {
                path: {
                    post_id: Number(postId),
                },
            });

            return response.data;
        },
    });

export const getPostsInfinite = (topicId: string) =>
    infiniteQueryOptions({
        queryKey: ['posts', topicId, 'infinite'],
        initialPageParam: 1,
        getNextPageParam: (lastPage: { posts: Post[]; has_more: boolean }, allPages) => {
            return lastPage.has_more ? allPages.length + 1 : undefined;
        },
        queryFn: async ({ pageParam }) => {
            const response = await useApi('/t/{topic_id}/posts', 'get', {
                path: {
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

export const usePosts = (topicId: string, page: number) => useQuery(getPosts(topicId, page));

export const usePost = (postId: string) => useQuery(getPost(postId));

export const usePostsInfinite = (topicId: string) => useInfiniteQuery(getPostsInfinite(topicId));
