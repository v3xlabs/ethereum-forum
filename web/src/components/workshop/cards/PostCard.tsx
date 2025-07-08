import React from 'react';
import { LuCalendar, LuExternalLink, LuUser } from 'react-icons/lu';

import { formatRelativeTime, getPlainText, truncateText } from '@/util/format';

import type { Post } from '../types';

interface PostCardProps {
    post: Post;
    showDetails?: boolean;
}

export const PostCard: React.FC<PostCardProps> = ({ post, showDetails = true }) => {
    const plainText = getPlainText(post.cooked);

    return (
        <div className="border border-primary/20 rounded-lg p-4 bg-secondary/50 hover:bg-secondary/70 transition-colors space-y-3">
            <div className="flex items-start justify-between gap-3">
                <div className="flex items-center gap-2">
                    <LuUser size={14} className="text-primary/60" />
                    <span className="font-medium text-primary text-sm">@{post.username}</span>
                    {post.name && <span className="text-primary/60 text-xs">({post.name})</span>}
                </div>
                <div className="flex items-center gap-2">
                    {showDetails && (
                        <span className="text-xs text-primary/60">#{post.post_number}</span>
                    )}
                    <a
                        href={`/t/magicians/${post.topic_id}/#${post.post_number}`}
                        className="text-primary/60 hover:text-primary transition-colors"
                        title="View post"
                    >
                        <LuExternalLink size={14} />
                    </a>
                </div>
            </div>

            <div className="text-sm text-primary/80 leading-relaxed">
                {truncateText(plainText, showDetails ? 300 : 150)}
            </div>

            {showDetails && (
                <div className="flex items-center gap-4 text-xs text-primary/60">
                    <div className="flex items-center gap-1">
                        <LuCalendar size={12} />
                        <span>{formatRelativeTime(post.created_at)}</span>
                    </div>
                    {post.like_count && post.like_count > 0 && (
                        <div className="flex items-center gap-1">
                            <span>❤️ {post.like_count}</span>
                        </div>
                    )}
                    {post.reply_count && post.reply_count > 0 && (
                        <div className="flex items-center gap-1">
                            <span>💬 {post.reply_count}</span>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
};
