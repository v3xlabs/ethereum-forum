import { Link } from '@tanstack/react-router';
import classNames from 'classnames';
import { parseISO } from 'date-fns';
import { FC } from 'react';
import { FiHeart, FiLink } from 'react-icons/fi';
import { LuHammer, LuShield } from 'react-icons/lu';
import { SiEthereum } from 'react-icons/si';

import { Post } from '@/api/topics';

import { TimeAgo } from '../TimeAgo';
import { Tooltip } from '../tooltip/Tooltip';
import { TrustLevel } from '../TrustLevel';
import { Prose } from './Prose';

export const TopicPost: FC<{ post: Post }> = ({ post }) => {
    const extra = post.extra as Record<string, unknown>;
    const displayName = (extra?.['display_username'] ||
        extra?.['name'] ||
        extra?.['username']) as string;
    const username = extra?.['username'] as string;
    const avatar = extra?.['avatar_template'] as string;
    const actions_summary = (extra?.['actions_summary'] as { id: number; count: number }[]) || [];
    // like count is action id 2 (my guess)
    const likes = actions_summary.find((action) => action.id === 2)?.count || 0;
    const post_url = post.post_url ?? '';
    const hidden = extra?.['hidden'] as boolean;
    const trustLevel = extra?.['trust_level'] as number;
    const isModerator = extra?.['moderator'] as boolean;
    const isAdmin = extra?.['admin'] as boolean;

    const actionCode = extra?.['action_code'] as string;

    return (
        <div
            key={post.post_id}
            id={`p-${post.post_id}`}
            className={classNames('post space-y-2', hidden && 'opacity-50')}
        >
            <div className="flex items-center justify-between gap-2">
                <Link
                    to={'/u/$userId'}
                    params={{
                        userId: username,
                    }}
                    className="flex items-center gap-2"
                >
                    {avatar && (
                        <img
                            src={'https://ethereum-magicians.org' + avatar.replace('{size}', '40')}
                            alt={username}
                            className="w-7 h-7 rounded-sm"
                        />
                    )}
                    <div className="leading-3">
                        <div className="font-bold">{displayName ?? post.user_id}</div>
                        {username.toLowerCase().replace(' ', '') !==
                            displayName.toLowerCase().replace(' ', '') && (
                            <div className="text-sm text-gray-500">@{username}</div>
                        )}
                    </div>
                    <div className="flex items-center gap-0.5 pl-1">
                        <TrustLevel trustLevel={trustLevel} />
                        {isModerator && (
                            <Tooltip
                                trigger={
                                    <button className="p-1 hover:bg-secondary rounded-sm text-sm">
                                        <LuShield />
                                    </button>
                                }
                            >
                                Moderator
                            </Tooltip>
                        )}
                        {isAdmin && (
                            <Tooltip
                                trigger={
                                    <button className="p-1 hover:bg-secondary rounded-sm text-sm">
                                        <LuHammer />
                                    </button>
                                }
                            >
                                Admin
                            </Tooltip>
                        )}
                    </div>
                </Link>
                <div className="text-end font-bold text-sm">
                    {post.updated_at ? <TimeAgo date={parseISO(post.updated_at)} /> : ''}
                </div>
            </div>
            {post.cooked && (
                <Prose content={post.cooked} topicId={post.topic_id} postId={post.post_id} />
            )}
            {actionCode && (
                <div className="text-sm border border-primary p-2">ACTION CODE: {actionCode}</div>
            )}
            <div className="flex items-center gap-2 justify-end">
                <div className="text-sm text-gray-500 flex items-center gap-1">
                    <div className="flex items-center gap-1">
                        <FiHeart /> {likes}
                    </div>
                    <a
                        href={'#p-' + post.post_id}
                        className="text-sm text-gray-500 flex items-center gap-1 hover:bg-secondary p-1"
                    >
                        <FiLink />
                    </a>
                    {post_url && (
                        <a
                            href={'https://ethereum-magicians.org' + post_url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-sm text-gray-500 flex items-center gap-1 hover:bg-secondary p-1"
                        >
                            <SiEthereum />
                        </a>
                    )}
                </div>
            </div>
        </div>
    );
};
