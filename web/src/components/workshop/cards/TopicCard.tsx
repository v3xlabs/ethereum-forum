import { Link } from '@tanstack/react-router';
import React from 'react';
import { LuCalendar, LuEye, LuMessageSquare } from 'react-icons/lu';

import { formatCompact, formatRelativeTime } from '@/util/format';

import type { TopicSummary } from '../types';

interface TopicCardProps {
    topic: TopicSummary;
    showDetails?: boolean;
}

export const TopicCard: React.FC<TopicCardProps> = ({ topic, showDetails = true }) => {
    return (
        <Link
            to="/t/$discourseId/$topicId"
            params={{
                discourseId: topic.discourse_id ?? 'magicians',
                topicId: topic.topic_id.toString(),
            }}
            className="block border border-primary/20 rounded-lg p-4 bg-secondary/50 hover:bg-secondary/70 transition-colors space-y-3"
            title="View topic"
        >
            {topic.title && (
                <h3 className="font-semibold text-primary text-sm leading-tight mb-2 line-clamp-2">
                    {topic.title}
                </h3>
            )}
            {showDetails && (
                <div className="flex items-center gap-4 text-xs text-primary/60">
                    {topic.post_count && (
                        <div className="flex items-center gap-1">
                            <LuMessageSquare size={12} />
                            <span>{topic.post_count} posts</span>
                        </div>
                    )}
                    {topic.created_at && (
                        <div className="flex items-center gap-1">
                            <LuCalendar size={12} />
                            <span>{formatRelativeTime(topic.created_at)}</span>
                        </div>
                    )}

                    {topic.view_count && (
                        <div className="flex items-center gap-1">
                            <LuEye size={12} />
                            <span>{formatCompact(topic.view_count)} views</span>
                        </div>
                    )}
                </div>
            )}
        </Link>
    );
};
