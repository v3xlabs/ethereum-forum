import React from 'react';
import { LuCog, LuLoader } from 'react-icons/lu';

import { useTopic } from '@/api/topics';
import { Tooltip } from '@/components/tooltip/Tooltip';

// Topic Preview Tooltip Component
export const TopicPreviewTooltip = ({
    instance,
    topicId,
    children,
}: {
    instance: 'magicians' | 'research';
    topicId: string;
    children: React.ReactNode;
}) => {
    const { data: topic, isLoading, error } = useTopic(instance, topicId);

    const tooltipContent = () => {
        if (isLoading) {
            return (
                <div className="flex items-center gap-2 p-2">
                    <LuLoader className="animate-spin" size={16} />
                    <span>Loading...</span>
                </div>
            );
        }

        if (error || !topic) {
            return (
                <div className="flex items-center gap-2 p-2 text-red-500">
                    <LuCog size={16} />
                    <span>Topic not found</span>
                </div>
            );
        }

        return (
            <div className="p-3 max-w-sm">
                <div className="mb-2">
                    <div className="font-semibold text-sm line-clamp-2">{topic.title}</div>
                    <div className="text-xs text-gray-500 mt-1">#{topic.topic_id}</div>
                </div>

                <div className="text-xs text-gray-600 space-y-1">
                    <div className="flex gap-4">
                        <span>
                            {topic.post_count} post{topic.post_count !== 1 ? 's' : ''}
                        </span>
                        <span>
                            {topic.view_count} view{topic.view_count !== 1 ? 's' : ''}
                        </span>
                    </div>

                    {topic.like_count > 0 && (
                        <div>
                            {topic.like_count} like{topic.like_count !== 1 ? 's' : ''}
                        </div>
                    )}

                    <div className="flex gap-4 text-xs">
                        <span>Created: {new Date(topic.created_at).toLocaleDateString()}</span>
                        {topic.last_post_at && (
                            <span>
                                Last post: {new Date(topic.last_post_at).toLocaleDateString()}
                            </span>
                        )}
                    </div>

                    {topic.pm_issue && (
                        <div className="text-blue-600 font-medium">PM Issue #{topic.pm_issue}</div>
                    )}
                </div>
            </div>
        );
    };

    return <Tooltip trigger={<span>{children}</span>}>{tooltipContent()}</Tooltip>;
};
