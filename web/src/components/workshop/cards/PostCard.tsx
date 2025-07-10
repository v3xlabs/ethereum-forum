import { Link } from '@tanstack/react-router';
import React from 'react';
import { LuCalendar, LuUser } from 'react-icons/lu';

import { formatRelativeTime, getPlainText, truncateText } from '@/util/format';

import type { Post, SearchEntity } from '../types';

interface PostCardProps {
    post: Post;
    showDetails?: boolean;
    entity: SearchEntity;
}

export const PostCard: React.FC<PostCardProps> = ({ post, entity, showDetails = true }) => {
    if (!entity.cooked) return null;

    const plainText = getPlainText(entity.cooked);

    return (
        <Link
            to="/t/$discourseId/$topicId"
            params={{
                discourseId: entity.discourse_id ?? 'magicians',
                topicId: post.topic_id.toString(),
            }}
            className="block border border-primary/20 rounded-lg p-4 bg-secondary/50 hover:bg-secondary/70 transition-colors space-y-3"
            title="View post"
        >
            <div className="flex items-center gap-2">
                <LuUser size={14} className="text-primary/60" />
                <span className="font-medium text-primary text-sm">@{entity.username}</span>

                {/* doesn't return anything but should return user name (not @) if different */}
                {post.name && <span className="text-primary/60 text-xs">({post.name})</span>}
            </div>

            {plainText && (
                <div className="text-sm text-primary/80 leading-relaxed">
                    {truncateText(plainText, showDetails ? 300 : 150)}
                </div>
            )}

            {showDetails && (
                <div className="flex items-center gap-4 text-xs text-primary/60">
                    <div className="flex items-center gap-1">
                        <LuCalendar size={12} />
                        {post.created_at && <span>{formatRelativeTime(post.created_at)}</span>}
                    </div>
                </div>
            )}
        </Link>
    );
};
