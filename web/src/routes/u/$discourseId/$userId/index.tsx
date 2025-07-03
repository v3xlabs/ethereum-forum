import { createFileRoute, Link } from '@tanstack/react-router';
import { FC } from 'react';
import { FiHeart, FiMessageSquare } from 'react-icons/fi';

import { useUser, useUserSummary } from '@/api/user';
import { CategoryTag } from '@/components/CategoryTag';
import { TimeAgo } from '@/components/TimeAgo';
import { Tooltip } from '@/components/tooltip/Tooltip';
import { mapDiscourseInstanceUrl } from '@/util/discourse';
import { formatBigNumber } from '@/util/numbers';

const RouteComponent: FC = () => {
    const { discourseId, userId } = Route.useParams();

    const { data: userData, isLoading: userLoading } = useUser(discourseId, userId);
    const { data: userSummary, isLoading: summaryLoading } = useUserSummary(discourseId, userId);

    if (userLoading || summaryLoading) {
        return (
            <div className="mx-auto w-full max-w-screen-lg px-2 space-y-6">
                <div className="flex items-center gap-6 py-4">
                    <div className="size-20 rounded-full bg-gray-200 animate-pulse" />
                    <div className="flex flex-col justify-center">
                        <div className="h-8 w-48 bg-gray-200 animate-pulse mb-2" />
                        <div className="h-4 w-32 bg-gray-200 animate-pulse" />
                    </div>
                </div>
                <div className="flex flex-wrap text-sm text-primary border-b pb-2 border-primary gap-x-6 font-thin">
                    {Array.from({ length: 4 }).map((_, i) => (
                        <div key={i} className="h-4 w-32 bg-gray-200 animate-pulse" />
                    ))}
                </div>
                <div className="space-y-4">
                    <div className="h-6 w-48 bg-gray-200 animate-pulse" />
                    <div className="h-6 w-full bg-gray-200 animate-pulse" />
                    <div className="h-6 w-full bg-gray-200 animate-pulse" />
                </div>
            </div>
        );
    }

    if (!userData || !userSummary) {
        return (
            <div className="mx-auto w-full max-w-screen-lg pt-8 px-2 space-y-6">
                <div className="text-center text-gray-500">
                    User not found or no data available.
                </div>
            </div>
        );
    }

    const hasVanityName =
        userData.user.name &&
        userData.user.name.toLowerCase() !== userData.user.username.toLowerCase();

    const modifyAvatarUrl = (url: string | undefined) => {
        if (!url) return '';

        return mapDiscourseInstanceUrl(discourseId) + url.replace('{size}', '200');
    };

    return (
        <div className="mx-auto w-full max-w-screen-lg px-2 pb-8 space-y-6">
            <div className="flex items-center gap-6 py-4">
                <div className="size-20 rounded-full overflow-hidden border-4 border-gray-200 shadow">
                    <img
                        src={modifyAvatarUrl(userData.user.avatar_template)}
                        alt={`${userId} avatar`}
                        className="object-cover w-full h-full"
                    />
                </div>
                <div className="flex flex-col justify-center">
                    <h1 className="text-3xl">
                        {hasVanityName ? userData.user.name : userData.user.username}
                    </h1>
                    {hasVanityName && (
                        <span className="text-secondary">@{userData.user.username}</span>
                    )}
                </div>
            </div>

            <div className="flex flex-wrap text-sm text-primary border-b pb-2 border-primary gap-x-6 font-thin">
                {userData.user.created_at && (
                    <span>
                        Joined:{' '}
                        <Tooltip
                            trigger={
                                <span className="font-semibold">
                                    <TimeAgo date={new Date(userData.user.created_at)} />
                                </span>
                            }
                        >
                            {userData.user.created_at}
                        </Tooltip>
                    </span>
                )}
                <span>
                    Last post:{' '}
                    {userData.user.last_posted_at ? (
                        <Tooltip
                            trigger={
                                <span className="font-semibold">
                                    <TimeAgo date={new Date(userData.user.last_posted_at)} />
                                </span>
                            }
                        >
                            {userData.user.last_posted_at}
                        </Tooltip>
                    ) : (
                        'Never'
                    )}
                </span>
                {userData.user.profile_view_count !== undefined && (
                    <span>
                        Profile views:{' '}
                        <span className="font-semibold">{userData.user.profile_view_count}</span>
                    </span>
                )}
                {userSummary?.user_summary?.likes_given !== undefined && (
                    <span>
                        Likes given:{' '}
                        <span className="font-semibold">
                            {userSummary.user_summary.likes_given}
                        </span>
                    </span>
                )}
                {userSummary?.user_summary?.likes_received !== undefined && (
                    <span>
                        Likes received:{' '}
                        <span className="font-semibold">
                            {userSummary.user_summary.likes_received}
                        </span>
                    </span>
                )}
            </div>

            {userSummary?.user_summary?.top_categories && (
                <div>
                    <h2 className="text-lg font-semibold mb-1">Top Categories</h2>
                    <div className="flex flex-wrap gap-2">
                        {userSummary.user_summary.top_categories.map((cat) => (
                            <CategoryTag key={cat.name} tag={cat.name} />
                        ))}
                    </div>
                </div>
            )}

            {(userSummary.user_summary?.most_liked_by_users?.length ?? 0) > 0 && (
                <div>
                    <h2 className="text-lg font-semibold mb-1">Most Liked By</h2>
                    <div className="flex flex-wrap gap-2 items-center">
                        {userSummary.user_summary?.most_liked_by_users?.map((u) => (
                            <div key={u.id} className="flex items-center gap-1">
                                <img
                                    src={modifyAvatarUrl(u.avatar_template)}
                                    alt={u.username}
                                    className="size-6 rounded-full"
                                />
                                <span className="text-xs">{u.username}</span>
                                <span className="text-xs text-gray-400">×{u.count}</span>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {(userSummary.user_summary?.most_liked_users?.length ?? 0) > 0 && (
                <div>
                    <h2 className="text-lg font-semibold mb-1">Most Liked Users</h2>
                    <div className="flex flex-wrap gap-2 items-center">
                        {userSummary.user_summary?.most_liked_users?.map((u) => (
                            <div key={u.id} className="flex items-center gap-1">
                                <img
                                    src={modifyAvatarUrl(u.avatar_template)}
                                    alt={u.username}
                                    className="size-6 rounded-full"
                                />
                                <span className="text-xs">{u.username}</span>
                                <span className="text-xs text-gray-400">×{u.count}</span>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {userSummary?.user_summary?.replies && userSummary.user_summary.replies.length > 0 && (
                <div>
                    <h2 className="text-lg font-semibold mb-1">Recent Replies</h2>
                    <ul className="list-disc pl-5">
                        {userSummary.user_summary.replies.map((reply, i) => {
                            const topic = userSummary.topics?.find((t) => t.id === reply.topic_id);

                            return (
                                <li key={i}>
                                    <Link
                                        to="/t/$discourseId/$topicId"
                                        params={{
                                            discourseId: discourseId,
                                            topicId: reply.topic_id.toString(),
                                        }}
                                        className="underline"
                                    >
                                        {topic?.title || `Topic #${reply.topic_id}`}
                                    </Link>
                                    <span className="ml-2 text-xs text-gray-500">
                                        {reply.like_count} likes
                                    </span>
                                </li>
                            );
                        })}
                    </ul>
                </div>
            )}

            <div className="flex flex-col gap-4">
                <h2 className="text-xl font-semibold">Topics</h2>
                <div className="flex flex-col gap-2">
                    {(!userSummary?.topics || userSummary.topics.length === 0) && (
                        <div className="text-gray-500 italic">No topics found.</div>
                    )}
                    {userSummary?.topics?.map((topic) => (
                        <Link
                            to="/t/$discourseId/$topicId"
                            params={{ discourseId: discourseId, topicId: topic.id.toString() }}
                            key={topic.id}
                            className="card hover:border-primary border border-transparent gap-1 flex flex-col"
                        >
                            <div className="grow space-y-1">
                                <div className="font-bold">{topic.title}</div>
                            </div>
                            <div className="flex justify-between items-start">
                                <div className="flex items-center gap-2 justify-start">
                                    <div className="flex items-center gap-1">
                                        <FiHeart />
                                        {formatBigNumber(topic?.like_count ?? 0)}
                                    </div>
                                    <div className="flex items-center gap-1">
                                        <FiMessageSquare />
                                        {formatBigNumber(topic?.posts_count ?? 0)}
                                    </div>
                                </div>
                            </div>
                        </Link>
                    ))}
                </div>
            </div>
        </div>
    );
};

export const Route = createFileRoute('/u/$discourseId/$userId/')({
    component: RouteComponent,
});
