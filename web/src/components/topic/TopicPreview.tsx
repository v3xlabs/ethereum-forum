import { Link } from '@tanstack/react-router';
import { parseISO } from 'date-fns';
import { FC } from 'react';
import { FiEye, FiHeart, FiMessageSquare } from 'react-icons/fi';

import { Topic } from '@/api/topics';
import { decodeCategory } from '@/util/category';
import { formatBigNumber } from '@/util/numbers';

import { TimeAgo } from '../TimeAgo';
import { Participant } from '@/types/topics';

export const TopicPreview: FC<{ topic: Topic }> = ({ topic }) => {
    const extra = (topic.extra || {}) as Record<string, unknown>;
    const tags = [
        ...decodeCategory(extra?.['category_id'] as number),
        ...(extra?.['tags'] as string[]),
    ];
    const participants = (extra?.details as Record<string, unknown>)?.participants as Participant[];

    return (
        <Link
            to="/t/$topicId"
            params={{ topicId: topic.topic_id.toString() }}
            className="card hover:border-primary border border-transparent gap-1 flex flex-col"
        >
            <div className="grow space-y-1">
                <div className="font-bold">{topic.title}</div>
                <div className="flex gap-2 whitespace-nowrap overflow-x-hidden">
                    {tags?.map((tag) => (
                        <div
                            key={tag}
                            className="text-sm text-gray-500 bg-primary px-1 border border-primary"
                        >
                            {tag}
                        </div>
                    ))}
                </div>
            </div>
            <div className="flex items-center justify-end gap-1 -space-x-2">
                {participants &&
                    participants.slice(0, 8).map((participant) => (
                        <div key={participant.id} className="flex items-center gap-2">
                            <div className="size-6">
                                <img
                                    src={
                                        'https://ethereum-magicians.org' +
                                        participant.avatar_template.replace('{size}', '40')
                                    }
                                    alt={participant.username}
                                    className="size-full rounded-md"
                                />
                            </div>
                        </div>
                    ))}
                {participants && participants.length > 8 && (
                    <div className="text-xs text-gray-500 bg-primary flex justify-center items-center h-full rounded-sm leading-4 aspect-square">
                        +{formatBigNumber(participants.length - 8)}
                    </div>
                )}
            </div>
            <div className="flex justify-between items-start">
                <div className="flex items-center gap-2 justify-start">
                    <div className="flex items-center gap-1">
                        <FiEye />
                        {formatBigNumber(topic?.view_count ?? 0)}
                    </div>
                    <div className="flex items-center gap-1">
                        <FiHeart />
                        {formatBigNumber(topic?.like_count ?? 0)}
                    </div>
                    <div className="flex items-center gap-1">
                        <FiMessageSquare />
                        {formatBigNumber(topic?.post_count ?? 0)}
                    </div>
                </div>

                <div>{topic.last_post_at && <TimeAgo date={parseISO(topic.last_post_at)} />}</div>
            </div>
        </Link>
    );
};
