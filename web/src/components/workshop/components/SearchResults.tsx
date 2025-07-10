import classNames from 'classnames';
import React, { useState } from 'react';
import { LuChevronDown, LuChevronLeft, LuHash, LuMessageSquare, LuSearch } from 'react-icons/lu';

import { usePosts, useTopic } from '@/api';

import { PostCard } from '../cards/PostCard';
import { TopicCard } from '../cards/TopicCard';
import type { Post, SearchEntity, SearchResult, TopicSummary } from '../types';

interface SearchResultsProps {
    data: SearchResult | SearchEntity[];
    toolName: string;
}

// Helper to get results summary message
const getResultsMessage = (toolName: string, topicCount: number, postCount: number) => {
    const totalHits = topicCount + postCount;

    switch (toolName) {
        case 'search_topics':
            return `${topicCount} topic${topicCount !== 1 ? 's' : ''} found`;
        case 'search_posts':
            return `${postCount} post${postCount !== 1 ? 's' : ''} found`;
        case 'search_forum':
        default:
            return `${totalHits} result${totalHits !== 1 ? 's' : ''} found`;
    }
};

const Topics: React.FC<{ entity: SearchEntity }> = ({ entity }) => {
    // basic implementation, will be changed to ts pattern ?
    const {
        data: topic,
        isLoading,
        isError,
        error,
    } = useTopic(entity.discourse_id ?? 'magicians', (entity.topic_id ?? 0).toString());

    if (!topic) return <span className="text-red-600">Topic not found.</span>;

    if (isError) return <span className="text-red-600">Error: {error.message}</span>;

    if (isLoading) return <span>Loading...</span>;

    return <TopicCard topic={topic as TopicSummary} />;
};

const Posts: React.FC<{ entity: SearchEntity }> = ({ entity }) => {
    const {
        data: postData,
        isLoading,
        isError,
        error,
    } = usePosts(entity.discourse_id ?? 'magicians', (entity.topic_id ?? 0).toString(), 1);

    const post = postData?.posts.find((p) => p.topic_id === entity.topic_id);

    if (!post) return <span className="text-red-600">Post not found.</span>;

    if (isError) return <span className="text-red-600">Error: {error.message}</span>;

    if (isLoading) return <span>Loading...</span>;

    return <PostCard post={post as Post} entity={entity} />;
};

export const SearchResults: React.FC<SearchResultsProps> = ({ data, toolName }) => {
    // Individual expansion states for each section
    const [isTopicsExpanded, setIsTopicsExpanded] = useState(false);
    const [isPostsExpanded, setIsPostsExpanded] = useState(false);

    // Filter topic & post search result entities
    const entities: SearchEntity[] = Array.isArray(data) ? data : [];

    const topics = entities.filter((entity) => entity.entity_type === 'topic');
    const posts = entities.filter((entity) => entity.entity_type === 'post');

    const topicCount = topics.length;
    const postCount = posts.length;

    // Determine display logic similar to get_posts implementation
    const hasManyTopics = topicCount >= 4;
    const hasManyPosts = postCount >= 4;

    // For topics: show first 3 when collapsed and many topics exist
    const topicsToShow = hasManyTopics && !isTopicsExpanded ? topics.slice(0, 3) : topics;

    // For posts: show first 3 when collapsed and many posts exist
    const postsToShow = hasManyPosts && !isPostsExpanded ? posts.slice(0, 3) : posts;

    return (
        <div className="space-y-4">
            {/* Results Summary */}
            <div className="flex items-center gap-2 p-3 bg-success/10 border border-success/30 rounded-lg">
                <LuSearch className="text-success" size={16} />
                <span className="text-success font-medium text-sm">
                    {getResultsMessage(toolName, topicCount, postCount)}
                </span>
            </div>

            {/* Topics Section */}
            {topics.length > 0 && (
                <div className="space-y-3">
                    <div className="flex items-center justify-between">
                        <h4 className="text-sm font-semibold text-primary/80 flex items-center gap-2">
                            <LuHash size={14} />
                            Topics ({topicCount})
                        </h4>
                        <button
                            onClick={() => setIsTopicsExpanded(!isTopicsExpanded)}
                            className="button aspect-square flex items-center justify-center"
                        >
                            {isTopicsExpanded ? (
                                <LuChevronDown size={14} />
                            ) : (
                                <LuChevronLeft size={14} />
                            )}
                        </button>
                    </div>

                    {/* Show content when expanded or when there aren't many topics */}
                    {isTopicsExpanded && (
                        <div
                            className={classNames(
                                'space-y-3 transition-all duration-300',
                                hasManyTopics ? 'max-h-80 overflow-y-auto' : ''
                            )}
                        >
                            {topicsToShow.map((topic) => (
                                <Topics key={topic.topic_id} entity={topic} />
                            ))}
                        </div>
                    )}
                </div>
            )}

            {/* Posts Section */}
            {posts.length > 0 && (
                <div className="space-y-3">
                    <div className="flex items-center justify-between">
                        <h4 className="text-sm font-semibold text-primary/80 flex items-center gap-2">
                            <LuMessageSquare size={14} />
                            Posts ({postCount})
                        </h4>
                        <button
                            onClick={() => setIsPostsExpanded(!isPostsExpanded)}
                            className="button aspect-square flex items-center justify-center"
                        >
                            {isPostsExpanded ? (
                                <LuChevronDown size={14} />
                            ) : (
                                <LuChevronLeft size={14} />
                            )}
                        </button>
                    </div>

                    {/* Show content when expanded or when there aren't many posts */}
                    {isPostsExpanded && (
                        <div
                            className={classNames(
                                'space-y-3 transition-all duration-300',
                                hasManyPosts ? 'max-h-80 overflow-y-auto' : ''
                            )}
                        >
                            {postsToShow.map((post) => (
                                <Posts key={post.post_id} entity={post} />
                            ))}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
};
