import React from 'react';
import { LuCalendar, LuExternalLink, LuMessageSquare, LuUsers } from 'react-icons/lu';

import { formatRelativeTime } from '@/util/format';

import type { TopicSummary } from '../types';

interface TopicCardProps {
    topic: TopicSummary;
    showDetails?: boolean;
}

export const TopicCard: React.FC<TopicCardProps> = ({ topic, showDetails = true }) => (
    <div className="border border-primary/20 rounded-lg p-4 bg-secondary/50 hover:bg-secondary/70 transition-colors">
        <div className="flex items-start justify-between gap-3">
            <div className="flex-1 min-w-0">
                <h3 className="font-semibold text-primary text-sm leading-tight mb-2 line-clamp-2">
                    {topic.title}
                </h3>
                {showDetails && (
                    <div className="flex items-center gap-4 text-xs text-primary/60">
                        <div className="flex items-center gap-1">
                            <LuMessageSquare size={12} />
                            <span>{topic.posts_count} posts</span>
                        </div>
                        <div className="flex items-center gap-1">
                            <LuCalendar size={12} />
                            <span>{formatRelativeTime(topic.created_at)}</span>
                        </div>
                        {topic.views && (
                            <div className="flex items-center gap-1">
                                <span>{topic.views} views</span>
                            </div>
                        )}
                    </div>
                )}
            </div>
            <a
                // should not be hard coded
                href={`/t/magicians/${topic.id}`}
                className="text-primary/60 hover:text-primary transition-colors flex-shrink-0"
                title="View topic"
            >
                <LuExternalLink size={16} />
            </a>
        </div>
        {topic.participants && topic.participants.length > 0 && (
            <div className="mt-3 pt-3 border-t border-primary/10">
                <div className="flex items-center gap-2 text-xs text-primary/60">
                    <LuUsers size={12} />
                    <span>Participants:</span>
                    <div className="flex items-center gap-1">
                        {topic.participants.slice(0, 3).map((participant) => (
                            <span key={participant.id} className="text-primary">
                                @{participant.username}
                            </span>
                        ))}
                        {topic.participants.length > 3 && (
                            <span className="text-primary/50">
                                +{topic.participants.length - 3} more
                            </span>
                        )}
                    </div>
                </div>
            </div>
        )}
    </div>
);
