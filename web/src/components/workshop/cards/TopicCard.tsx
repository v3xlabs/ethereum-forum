import { Link } from '@tanstack/react-router';
import { FC } from 'react';
import { LuEye, LuMessageSquare } from 'react-icons/lu';

import { CategoryTag } from '@/components/CategoryTag';
import { DiscourseInstanceIcon } from '@/components/DiscourseInstanceIcon';
import { decodeCategory } from '@/util/category';
import { formatCompact, formatRelativeTime } from '@/util/format';

import type { TopicSummary } from '../types';

interface TopicCardProps {
    topic: TopicSummary;
    showDetails?: boolean;
}

export const TopicCard: FC<TopicCardProps> = ({ topic, showDetails = true }) => {
    const extra = (topic.extra || {}) as Record<string, unknown>;

    const tags = [
        ...decodeCategory(extra?.['category_id'] as number),
        ...(extra?.['tags'] as string[]),
    ];

    return (
        <Link
            to="/t/$discourseId/$topicId"
            params={{
                discourseId: topic.discourse_id ?? 'magicians',
                topicId: topic.topic_id.toString(),
            }}
            className="block"
            title="View topic"
        >
            <div className="border border-primary/20 rounded-lg p-4 bg-secondary/50 hover:bg-secondary/70 transition-colors space-y-3">
                <div className="flex items-start gap-4 justify-between">
                    {topic.title && (
                        <h3 className="font-semibold text-primary text-sm leading-tight mb-2 line-clamp-2">
                            {topic.title}
                        </h3>
                    )}
                    {topic.discourse_id && (
                        <div>
                            <DiscourseInstanceIcon discourse_id={topic.discourse_id} />
                        </div>
                    )}
                </div>

                {tags && (
                    <span className="flex gap-2 whitespace-nowrap overflow-x-hidden">
                        {tags?.map((tag) => <CategoryTag key={tag} tag={tag} />)}
                    </span>
                )}

                {showDetails && (
                    <div className="flex items-center justify-between gap-4 text-xs text-primary/60">
                        <div className="flex gap-2">
                            {topic.view_count && (
                                <span className="flex items-center gap-1">
                                    <LuEye size={12} />
                                    {formatCompact(topic.view_count)}
                                </span>
                            )}
                            {topic.post_count && (
                                <span className="flex items-center gap-1">
                                    <LuMessageSquare size={12} />
                                    {topic.post_count}
                                </span>
                            )}
                        </div>

                        {topic.created_at && <span>{formatRelativeTime(topic.created_at)}</span>}
                    </div>
                )}
            </div>
        </Link>
    );
};
