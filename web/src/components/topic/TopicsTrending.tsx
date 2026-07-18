import { FC, useRef } from 'react';
import { FiChevronLeft, FiChevronRight } from 'react-icons/fi';

import { useTopicsTrending } from '@/api/topics';

import { MicroInfo } from '../tooltip/MicroInfo';
import { TopicPreview } from './TopicPreview';
export const TopicsTrending: FC = () => {
    const { data, isLoading } = useTopicsTrending();
    const trackReference = useRef<HTMLDivElement>(null);

    const scrollTrack = (direction: 'left' | 'right') => {
        trackReference.current?.scrollBy({
            left: direction === 'left' ? -360 : 360,
            behavior: 'smooth',
        });
    };

    if (isLoading) {
        return <div>Loading...</div>;
    }

    return (
        <section className="space-y-3">
            <div className="flex items-baseline justify-between border-b border-b-primary pb-2">
                <div className="flex items-center gap-2 text-lg font-bold">
                    <span>Trending now</span>
                    <MicroInfo>
                        <div>
                            Trending topics are (currently) defined as the topics with the{' '}
                            <b>most posts</b> in the last 7 days
                        </div>
                    </MicroInfo>
                </div>
                <div className="flex gap-1">
                    <button
                        type="button"
                        className="button aspect-square button-ghost"
                        onClick={() => scrollTrack('left')}
                        aria-label="Show previous trending threads"
                    >
                        <FiChevronLeft />
                    </button>
                    <button
                        type="button"
                        className="button aspect-square button-ghost"
                        onClick={() => scrollTrack('right')}
                        aria-label="Show more trending threads"
                    >
                        <FiChevronRight />
                    </button>
                </div>
            </div>
            <div
                ref={trackReference}
                className="trending-track flex snap-x snap-mandatory gap-3 overflow-x-auto scroll-smooth pb-2"
            >
                {data
                    ?.slice(0, 6)
                    .sort(
                        (a, b) =>
                            new Date(b.last_post_at ?? '').getTime() -
                            new Date(a.last_post_at ?? '').getTime()
                    )
                    .map((topic) => (
                        <div
                            key={topic.topic_id}
                            className="flex w-[min(22rem,calc(100vw-2rem))] shrink-0 snap-start md:w-[calc((100%-1.5rem)/3)]"
                        >
                            <TopicPreview topic={topic} />
                        </div>
                    ))}
            </div>
        </section>
    );
};
