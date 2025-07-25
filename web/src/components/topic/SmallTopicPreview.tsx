import { Link } from '@tanstack/react-router';
import { FC } from 'react';

import { Topic } from '@/api/topics';
import { decodeCategory } from '@/util/category';

import { CategoryTag } from '../CategoryTag';
import { DiscourseInstanceIcon } from '../DiscourseInstanceIcon';

export const SmallTopicPreview: FC<{ topic: Topic }> = ({ topic }) => {
    const extra = (topic.extra || {}) as Record<string, unknown>;
    const tag = [
        ...decodeCategory(extra?.['category_id'] as number),
        ...(extra?.['tags'] as string[]),
        "no-category",
    ][0]

    return (
        <Link
            to="/t/$discourseId/$topicId"
            params={{ discourseId: topic.discourse_id, topicId: topic.topic_id.toString() }}
            className="flex items-start justify-between gap-2 py-1"
        >
            {/* Wide layout - inline */}
            <div className="flex-1 min-w-0 hidden @[28rem]:block">
                <div className="flex items-center gap-2">
                    {tag && (
                        <div className="w-36 flex justify-end flex-shrink-0">
                            <CategoryTag tag={tag} />
                        </div>
                    )}
                    <div className="font-bold">
                        {topic.title}
                    </div>
                </div>
            </div>
            
            {/* Narrow layout - stacked */}
            <div className="flex-1 min-w-0 @[28rem]:hidden space-y-1">
                {tag && (
                    <div className="flex justify-start">
                        <CategoryTag tag={tag} />
                    </div>
                )}
                <div className="font-bold">
                    {topic.title}
                </div>
            </div>
            
            <DiscourseInstanceIcon discourse_id={topic.discourse_id} />
        </Link>
    );
};
