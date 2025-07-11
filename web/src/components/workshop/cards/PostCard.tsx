import { Link } from '@tanstack/react-router';
import { FC } from 'react';

import { DiscourseInstanceIcon } from '@/components/DiscourseInstanceIcon';
import { mapDiscourseInstanceUrl } from '@/util/discourse';
import { formatRelativeTime, getPlainText, truncateText } from '@/util/format';

import type { Post, SearchEntity } from '../types';

interface PostCardProps {
    post: Post;
    showDetails?: boolean;
    entity: SearchEntity;
}

export const PostCard: FC<PostCardProps> = ({ post, entity, showDetails = true }) => {
    const plainText = getPlainText(entity.cooked ?? '');

    // copied from TopicPost
    const extra = post.extra as Record<string, unknown>;
    const displayName =
        (extra?.['display_username'] as string) ||
        (extra?.['name'] as string) ||
        (extra?.['username'] as string);
    const avatar = extra?.['avatar_template'] as string;
    const username = extra?.['username'] as string;

    return (
        <Link
            to="/t/$discourseId/$topicId"
            params={{
                discourseId: post.discourse_id ?? 'magicians',
                topicId: post.topic_id.toString(),
            }}
            className="block"
            title="View post"
        >
            <div className="border border-primary/20 rounded-lg p-4 bg-secondary/50 hover:bg-secondary/70 transition-colors space-y-3">
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                        {avatar && (
                            <img
                                src={
                                    mapDiscourseInstanceUrl(post.discourse_id) +
                                    avatar.replace('{size}', '40')
                                }
                                alt={username}
                                className="w-7 h-7 rounded-sm"
                            />
                        )}
                        <div className="flex items-center gap-1">
                            <span className="font-bold text-primary text-sm">@{displayName}</span>

                            {username && username?.toLowerCase() !== displayName.toLowerCase() && (
                                <span className="text-primary/60 text-xs">({username})</span>
                            )}
                        </div>
                    </div>
                    {post.discourse_id && (
                        <div>
                            <DiscourseInstanceIcon discourse_id={post.discourse_id} />
                        </div>
                    )}
                </div>

                {showDetails && (
                    <>
                        {plainText && (
                            <div className="text-sm text-primary/80 leading-relaxed">
                                {truncateText(plainText, showDetails ? 300 : 150)}
                            </div>
                        )}
                        {post.created_at && (
                            <div className="text-xs text-primary/60 text-right">
                                {formatRelativeTime(post.created_at)}
                            </div>
                        )}
                    </>
                )}
            </div>
        </Link>
    );
};
