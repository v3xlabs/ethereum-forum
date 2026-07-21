import React from 'react';
import { LuList, LuMessageCircle, LuUsers } from 'react-icons/lu';

import { useCachedTopicSummary } from '@/api/topics';

import { SummaryTabId, tryParseSummary } from './StreamingSummary';

// Renders nothing when no cached summary exists. The slide-over itself is
// owned by the page, not this rail, so it survives this block appearing or
// disappearing mid-generation.
export const SummaryRail: React.FC<{
    discourseId: string;
    topicId: number;
    onOpenSection: (tab: SummaryTabId) => void;
}> = ({ discourseId, topicId, onOpenSection }) => {
    const { data: cached } = useCachedTopicSummary(discourseId, topicId);

    const parsed = React.useMemo(
        () => (cached ? tryParseSummary(cached.summary_text) : null),
        [cached]
    );

    if (!parsed) return null;

    const perspectiveGroups = (parsed.perspectives ?? []).filter(
        (group) => (group.people?.length ?? 0) > 0
    );

    const rows = [
        {
            section: 'key_points' as const,
            label: 'Key points',
            icon: <LuList className="size-3.5" />,
            count: (parsed.key_points ?? []).length,
        },
        {
            section: 'open_questions' as const,
            label: 'Open questions',
            icon: <LuMessageCircle className="size-3.5" />,
            count: (parsed.open_questions ?? []).length,
        },
        {
            section: 'perspectives' as const,
            label: 'Perspectives',
            icon: <LuUsers className="size-3.5" />,
            count: perspectiveGroups.reduce((sum, group) => sum + (group.people?.length ?? 0), 0),
        },
    ].filter((row) => row.count > 0);

    if (rows.length === 0) return null;

    return (
        <div className="space-y-1.5">
            <div className="px-1.5">
                <h3 className="font-bold w-full border-b border-b-primary pb-1">Summary</h3>
            </div>
            <ul>
                {rows.map((row) => (
                    <li key={row.section}>
                        <button
                            className="w-full flex items-center gap-1 px-1.5 justify-between hover:bg-secondary transition-colors"
                            onClick={() => onOpenSection(row.section)}
                        >
                            <span className="text-base flex items-center gap-1.5">
                                <span className="text-primary/50">{row.icon}</span>
                                {row.label}
                            </span>
                            <span className="text-sm text-primary/50">{row.count}</span>
                        </button>
                    </li>
                ))}
            </ul>
        </div>
    );
};
