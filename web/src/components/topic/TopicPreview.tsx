import { Link } from '@tanstack/react-router';
import { parseISO } from 'date-fns';
import { FC } from 'react';
import { FiEye, FiHeart, FiMessageSquare } from 'react-icons/fi';

import { Topic } from '@/api/topics';
import { decodeCategory } from '@/util/category';
import { mapDiscourseInstanceUrl } from '@/util/discourse';
import { formatBigNumber } from '@/util/numbers';

import { CategoryTag } from '../CategoryTag';
import { DiscourseInstanceIcon } from '../DiscourseInstanceIcon';
import { TimeAgo } from '../TimeAgo';

type Participant = {
    id: number;
    username: string;
    avatar_template: string;
};

type TopicPreviewProps = {
    topic: Topic;
    variant?: 'card' | 'row';
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === 'object' && value !== null;

const isParticipant = (value: unknown): value is Participant => {
    if (!isRecord(value)) {
        return false;
    }

    return (
        typeof value.id === 'number' &&
        typeof value.username === 'string' &&
        typeof value.avatar_template === 'string'
    );
};

const getTopicTags = (extra: unknown) => {
    if (!isRecord(extra)) {
        return [];
    }

    const { category_id: categoryId, tags } = extra;

    return [
        ...(typeof categoryId === 'number' ? decodeCategory(categoryId) : []),
        ...(Array.isArray(tags)
            ? tags.filter((tag): tag is string => typeof tag === 'string')
            : []),
    ];
};

const getParticipants = (extra: unknown) => {
    if (
        !isRecord(extra) ||
        !isRecord(extra.details) ||
        !Array.isArray(extra.details.participants)
    ) {
        return [];
    }

    return extra.details.participants.filter(isParticipant);
};

const TopicParticipants = ({
    topic,
    participants,
}: {
    topic: Topic;
    participants: Participant[];
}) => {
    if (participants.length === 0) {
        return <span className="text-xs text-primary/50">-</span>;
    }

    return (
        <div className="flex items-center justify-end -space-x-2">
            {participants.slice(0, 4).map((participant) => (
                <img
                    key={participant.id}
                    src={
                        mapDiscourseInstanceUrl(topic.discourse_id) +
                        participant.avatar_template.replace('{size}', '40')
                    }
                    alt={participant.username}
                    className="size-6 rounded-full border-2 border-primary bg-secondary"
                />
            ))}
            {participants.length > 4 && (
                <span className="flex size-6 items-center justify-center rounded-full border-2 border-primary bg-tertiary text-[10px] font-bold">
                    +{formatBigNumber(participants.length - 4)}
                </span>
            )}
        </div>
    );
};

export const TopicPreview: FC<TopicPreviewProps> = ({ topic, variant = 'card' }) => {
    const tags = getTopicTags(topic.extra);
    const participants = getParticipants(topic.extra);

    if (variant === 'row') {
        return (
            <Link
                to="/t/$discourseId/$topicId"
                params={{ discourseId: topic.discourse_id, topicId: topic.topic_id.toString() }}
                className="group grid grid-cols-[auto_minmax(0,1fr)_auto] gap-x-3 border-b border-primary/50 px-3 py-4 transition-colors hover:bg-secondary sm:h-[68px] sm:grid-cols-[auto_minmax(0,1fr)_118px_92px_118px] sm:items-center sm:py-3"
            >
                <div className="mt-1 sm:mt-0">
                    <DiscourseInstanceIcon discourse_id={topic.discourse_id} />
                </div>
                <div className="min-w-0">
                    <div className="flex min-w-0 items-center gap-2">
                        <div
                            title={topic.title}
                            className="min-w-0 grow truncate font-bold group-hover:text-secondary"
                        >
                            {topic.title}
                        </div>
                        {tags.length > 0 && (
                            <div className="hidden max-w-[38%] shrink-0 justify-end gap-1 overflow-hidden sm:flex">
                                {tags.slice(0, 2).map((tag) => (
                                    <CategoryTag key={tag} tag={tag} />
                                ))}
                            </div>
                        )}
                    </div>
                    {tags.length > 0 && (
                        <div className="flex gap-1 overflow-hidden sm:hidden">
                            {tags.slice(0, 3).map((tag) => (
                                <CategoryTag key={tag} tag={tag} />
                            ))}
                        </div>
                    )}
                    <div className="flex items-center gap-3 pt-1 text-xs text-primary/60 sm:hidden">
                        <span className="flex items-center gap-1">
                            <FiHeart />
                            {formatBigNumber(topic.like_count)}
                        </span>
                        <span className="flex items-center gap-1">
                            <FiMessageSquare />
                            {formatBigNumber(topic.post_count)}
                        </span>
                        <span className="flex items-center gap-1">
                            <FiEye />
                            {formatBigNumber(topic.view_count)}
                        </span>
                    </div>
                </div>
                <div className="col-start-3 row-start-1 text-right text-xs text-primary/60 sm:col-start-3 sm:row-auto sm:flex sm:justify-center sm:gap-3">
                    <span className="hidden items-center gap-1 sm:flex">
                        <FiHeart />
                        {formatBigNumber(topic.like_count)}
                    </span>
                    <span className="hidden items-center gap-1 sm:flex">
                        <FiMessageSquare />
                        {formatBigNumber(topic.post_count)}
                    </span>
                    <span className="hidden items-center gap-1 sm:flex">
                        <FiEye />
                        {formatBigNumber(topic.view_count)}
                    </span>
                </div>
                <div className="hidden sm:col-start-4 sm:block">
                    <TopicParticipants topic={topic} participants={participants} />
                </div>
                <div className="hidden text-right text-xs text-primary/60 sm:col-start-5 sm:block">
                    {topic.last_post_at && <TimeAgo date={parseISO(topic.last_post_at)} />}
                </div>
            </Link>
        );
    }

    return (
        <Link
            to="/t/$discourseId/$topicId"
            params={{ discourseId: topic.discourse_id, topicId: topic.topic_id.toString() }}
            className="card hover:border-primary flex w-full flex-col gap-1 border border-transparent"
        >
            <div className="grow space-y-1">
                <div className="flex items-start gap-2 justify-between">
                    <div className="font-bold">{topic.title}</div>
                    <div>
                        <DiscourseInstanceIcon discourse_id={topic.discourse_id} />
                    </div>
                </div>
                <div className="flex gap-2 whitespace-nowrap overflow-x-hidden">
                    {tags?.map((tag) => <CategoryTag key={tag} tag={tag} />)}
                </div>
            </div>
            <TopicParticipants topic={topic} participants={participants} />
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
