import { FC, useState } from 'react';

import { Topic, useTopicsLatest } from '@/api/topics';
import { decodeCategory } from '@/util/category';

import { MicroInfo } from '../tooltip/MicroInfo';
import { TopicPreview } from './TopicPreview';

type ThreadFilter = {
    label: string;
    matches: (topic: Topic) => boolean;
};

const getTopicCategories = (extra: unknown) => {
    if (typeof extra !== 'object' || extra === null || !('category_id' in extra)) {
        return [];
    }

    const categoryId = extra.category_id;

    return typeof categoryId === 'number' ? decodeCategory(categoryId) : [];
};

const filters: ThreadFilter[] = [
    { label: 'All', matches: () => true },
    { label: 'Magicians', matches: (topic) => topic.discourse_id === 'magicians' },
    { label: 'Research', matches: (topic) => topic.discourse_id === 'research' },
    {
        label: 'Protocol Calls',
        matches: (topic) => getTopicCategories(topic.extra).includes('Protocol Calls'),
    },
];

export const TopicList: FC = () => {
    const { data, isLoading } = useTopicsLatest();
    const [selectedFilter, setSelectedFilter] = useState('All');

    const activeFilter = filters.find((filter) => filter.label === selectedFilter) ?? filters[0];
    const visibleTopics = data?.filter(activeFilter.matches);

    if (isLoading) {
        return <div>Loading...</div>;
    }

    return (
        <section>
            <div className="flex flex-col gap-3 border-b border-primary/70 py-4 sm:flex-row sm:items-center sm:justify-between">
                <div className="flex items-center gap-2 text-lg font-bold">
                    Latest threads
                    <MicroInfo>
                        <div>
                            Sorted by <b>latest activity</b> first
                        </div>
                    </MicroInfo>
                </div>
                <div className="flex flex-wrap gap-1" aria-label="Thread source filters">
                    {filters.map((filter) => (
                        <button
                            key={filter.label}
                            type="button"
                            onClick={() => setSelectedFilter(filter.label)}
                            className={`rounded-sm px-3 py-1 ring-1 text-sm transition-colors ${
                                selectedFilter === filter.label
                                    ? 'bg-primary text-secondary ring-primary'
                                    : 'text-primary/60 hover:bg-secondary hover:text-primary ring-secondary'
                            }`}
                        >
                            {filter.label}
                        </button>
                    ))}
                </div>
            </div>
            <div className="hidden grid-cols-[auto_minmax(0,1fr)_118px_92px_118px] gap-x-3 border-b border-primary/50 py-2 text-[11px] font-bold uppercase tracking-wider text-primary/50 sm:grid">
                <span />
                <span>Thread</span>
                <span className="text-center">Activity</span>
                <span className="text-right">Members</span>
                <span className="text-right">Last active</span>
            </div>
            <div>
                {visibleTopics?.map((topic) => (
                    <TopicPreview key={topic.topic_id} topic={topic} variant="row" />
                ))}
            </div>
        </section>
    );
};
