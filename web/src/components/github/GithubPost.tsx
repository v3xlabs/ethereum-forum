import { parseISO } from 'date-fns';
import { FC } from 'react';
import { FiGithub } from 'react-icons/fi';
import Markdown from 'react-markdown';

import { GithubIssueComment } from '@/types/github';

import { TimeAgo } from '../TimeAgo';

export const GithubPost: FC<{ post: GithubIssueComment }> = ({ post }) => {
    const reactions = post.reactions || {};
    const reactionList = [
        { key: '+1', emoji: '👍' },
        { key: '-1', emoji: '👎' },
        { key: 'laugh', emoji: '😄' },
        { key: 'hooray', emoji: '🎉' },
        { key: 'confused', emoji: '😕' },
        { key: 'heart', emoji: '❤️' },
        { key: 'rocket', emoji: '🚀' },
        { key: 'eyes', emoji: '👀' },
    ];

    return (
        <div className="post space-y-2">
            <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                    <img
                        src={post.user.avatar_url}
                        alt={post.user.login}
                        className="w-7 h-7 rounded-sm"
                    />
                    <div className="leading-3">
                        <div className="font-bold">{post.user.login}</div>
                    </div>
                </div>
                <div className="text-end font-bold text-sm">
                    {post.updated_at ? <TimeAgo date={parseISO(post.updated_at)} /> : ''}
                </div>
            </div>
            <div className="prose">
                <Markdown>{post.body}</Markdown>
            </div>
            <div className="flex items-center gap-2 justify-end">
                <div className="text-sm text-gray-500 flex items-center gap-1">
                    <div className="flex items-center gap-1">
                        {reactionList.map(
                            ({ key, emoji }) =>
                                Number(reactions[key as keyof typeof reactions]) > 0 && (
                                    <span key={key} className="flex items-center gap-0.5">
                                        {emoji} {reactions[key as keyof typeof reactions]}
                                    </span>
                                )
                        )}
                    </div>
                    <a
                        href={post.html_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm text-gray-500 flex items-center gap-1 hover:bg-secondary p-1"
                        title="View on GitHub"
                    >
                        <FiGithub />
                    </a>
                </div>
            </div>
        </div>
    );
};
