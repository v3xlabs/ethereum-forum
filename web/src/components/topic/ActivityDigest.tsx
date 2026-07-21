import { Link } from '@tanstack/react-router';
import { parseISO } from 'date-fns';
import { FC } from 'react';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import { useActivityDigest } from '@/api/topics';

import { TimeAgo } from '../TimeAgo';
import { MicroInfo } from '../tooltip/MicroInfo';

export const ActivityDigest: FC = () => {
    const { data: digest } = useActivityDigest();

    if (!digest) {
        return null;
    }

    return (
        <section className="space-y-3">
            <div className="flex items-baseline justify-between border-b border-b-primary pb-2">
                <div className="flex items-center gap-2 text-lg font-bold">
                    <span>What&apos;s new</span>
                    <MicroInfo>
                        <div>
                            A periodically generated digest of <b>recent activity</b> across the
                            forums
                        </div>
                    </MicroInfo>
                </div>
                <span className="text-sm text-primary/50">
                    <TimeAgo date={parseISO(digest.created_at)} />
                </span>
            </div>
            <div className="prose prose-sm max-w-none">
                <Markdown remarkPlugins={[remarkGfm]}>{digest.digest_text}</Markdown>
            </div>
            {digest.topics_included && digest.topics_included.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                    {digest.topics_included.map((topic) => (
                        <Link
                            key={`${topic.discourse_id}-${topic.topic_id}`}
                            to="/t/$discourseId/$topicId"
                            params={{
                                discourseId: topic.discourse_id,
                                topicId: topic.topic_id.toString(),
                            }}
                            title={topic.title}
                            className="max-w-72 truncate rounded-sm px-2 py-0.5 ring-1 ring-secondary text-sm text-primary/60 hover:bg-secondary hover:text-primary transition-colors"
                        >
                            {topic.title}
                        </Link>
                    ))}
                </div>
            )}
        </section>
    );
};
