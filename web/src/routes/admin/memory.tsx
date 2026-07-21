import { useQueryClient } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';
import { LuPencil, LuPlus, LuTrash2, LuX } from 'react-icons/lu';

import {
    parseMemorySources,
    useDeleteMemory,
    useMemory,
    useSnapshots,
    useStaging,
    useUpsertMemory,
} from '@/api/admin';
import { MemorySourceLinks } from '@/components/admin/MemorySourceLinks';
import { TimeAgo } from '@/components/TimeAgo';

type SourceRow = { url: string; reason: string };

const AdminMemoryPage = () => {
    const { data: memory, isLoading: isLoadingMemory } = useMemory();
    const { data: snapshots, isLoading: isLoadingSnapshots } = useSnapshots();
    const { data: staging } = useStaging();
    const deleteMutation = useDeleteMemory();
    const upsertMutation = useUpsertMemory();
    const queryClient = useQueryClient();
    const [term, setTerm] = useState('');
    const [content, setContent] = useState('');
    const [sourceRows, setSourceRows] = useState<SourceRow[]>([]);

    const updateSourceRow = (index: number, patch: Partial<SourceRow>) => {
        setSourceRows((rows) =>
            rows.map((row, rowIndex) => (rowIndex === index ? { ...row, ...patch } : row))
        );
    };

    const handleUpsert = () => {
        if (!term.trim() || !content.trim()) return;

        const sources = sourceRows
            .map((row) => ({ url: row.url.trim(), reason: row.reason.trim() }))
            .filter((row) => row.url.length > 0)
            .map((row) => ({ url: row.url, reason: row.reason.length > 0 ? row.reason : null }));

        upsertMutation.mutate(
            { term: term.trim(), content: content.trim(), sources },
            {
                onSuccess: () => {
                    setTerm('');
                    setContent('');
                    setSourceRows([]);
                    queryClient.invalidateQueries({ queryKey: ['admin', 'memory'] });
                },
            }
        );
    };

    return (
        <div className="space-y-6">
            <h1 className="text-2xl font-semibold">Shared Memory</h1>
            <p className="text-sm text-primary/60">
                Glossary entries injected into every LLM run (~4k token budget).
            </p>
            <div className="rounded border border-primary/20 p-4 space-y-3">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Add or Update Entry
                </h2>
                <div className="flex gap-2">
                    <input
                        type="text"
                        placeholder="Term"
                        className="flex-1 px-3 py-2 border border-primary/20 rounded bg-primary/5 text-sm"
                        value={term}
                        onChange={(event) => setTerm(event.target.value)}
                        onKeyDown={(event) => event.key === 'Enter' && handleUpsert()}
                    />
                    <input
                        type="text"
                        placeholder="Definition"
                        className="flex-[2] px-3 py-2 border border-primary/20 rounded bg-primary/5 text-sm"
                        value={content}
                        onChange={(event) => setContent(event.target.value)}
                        onKeyDown={(event) => event.key === 'Enter' && handleUpsert()}
                    />
                </div>
                {sourceRows.map((row, index) => (
                    <div key={index} className="flex gap-2">
                        <input
                            type="text"
                            placeholder="/t/magicians/1234 or https://eips.ethereum.org/..."
                            className="flex-[2] px-3 py-2 border border-primary/20 rounded bg-primary/5 text-sm"
                            value={row.url}
                            onChange={(event) =>
                                updateSourceRow(index, { url: event.target.value })
                            }
                        />
                        <input
                            type="text"
                            placeholder="Reason (optional)"
                            className="flex-1 px-3 py-2 border border-primary/20 rounded bg-primary/5 text-sm"
                            value={row.reason}
                            onChange={(event) =>
                                updateSourceRow(index, { reason: event.target.value })
                            }
                        />
                        <button
                            className="px-3 py-2 text-primary/40 hover:text-red-500 transition-colors"
                            title="Remove link"
                            onClick={() =>
                                setSourceRows((rows) =>
                                    rows.filter((_, rowIndex) => rowIndex !== index)
                                )
                            }
                        >
                            <LuX className="w-4 h-4" />
                        </button>
                    </div>
                ))}
                <div className="flex items-center justify-between gap-2">
                    <button
                        className="flex items-center gap-1.5 px-2 py-1 rounded text-xs text-primary/60 hover:text-primary hover:bg-primary/5 transition-colors"
                        onClick={() => setSourceRows((rows) => [...rows, { url: '', reason: '' }])}
                    >
                        <LuPlus className="w-3.5 h-3.5" />
                        Add link
                    </button>
                    <button
                        className="px-3 py-2 bg-primary/10 rounded text-sm hover:bg-primary/20 transition-colors disabled:opacity-50"
                        onClick={handleUpsert}
                        disabled={upsertMutation.isPending}
                    >
                        Save entry
                    </button>
                </div>
                <p className="text-xs text-primary/40">
                    Saving an existing term overwrites its definition and links.
                </p>
            </div>
            <div className="space-y-2">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Current Entries ({memory?.length ?? 0})
                </h2>
                {isLoadingMemory ? (
                    <div className="space-y-2">
                        {[0, 1, 2].map((index) => (
                            <div key={index} className="h-14 rounded bg-primary/5 animate-pulse" />
                        ))}
                    </div>
                ) : (
                    <div className="space-y-2">
                        {memory?.map((entry) => (
                            <div
                                key={entry.entry_id}
                                className="flex items-start gap-3 rounded border border-primary/20 p-3"
                            >
                                <div className="flex-1 min-w-0">
                                    <div className="font-medium text-sm">{entry.term}</div>
                                    <div className="text-sm text-primary/60 mt-1">
                                        {entry.content}
                                    </div>
                                    <MemorySourceLinks
                                        sources={parseMemorySources(entry.sources)}
                                    />
                                </div>
                                <button
                                    className="p-1 text-primary/40 hover:text-primary transition-colors"
                                    title="Edit entry"
                                    onClick={() => {
                                        setTerm(entry.term);
                                        setContent(entry.content);
                                        setSourceRows(
                                            parseMemorySources(entry.sources).map((source) => ({
                                                url: source.url,
                                                reason: source.reason ?? '',
                                            }))
                                        );
                                        window.scrollTo({ top: 0, behavior: 'smooth' });
                                    }}
                                >
                                    <LuPencil className="w-4 h-4" />
                                </button>
                                <button
                                    className="p-1 text-primary/40 hover:text-red-500 transition-colors"
                                    title="Delete entry"
                                    onClick={() =>
                                        deleteMutation.mutate(entry.entry_id, {
                                            onSuccess: () =>
                                                queryClient.invalidateQueries({
                                                    queryKey: ['admin', 'memory'],
                                                }),
                                        })
                                    }
                                >
                                    <LuTrash2 className="w-4 h-4" />
                                </button>
                            </div>
                        ))}
                        {(!memory || memory.length === 0) && (
                            <div className="text-sm text-primary/40">No entries yet.</div>
                        )}
                    </div>
                )}
            </div>
            <div className="space-y-2">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Staged Candidates ({staging?.length ?? 0})
                </h2>
                <p className="text-xs text-primary/40">
                    Proposed by summarizer/digest runs. The curator reviews and promotes (or
                    rejects) these on its next run.
                </p>
                <div className="space-y-2">
                    {staging?.map((entry) => (
                        <div
                            key={entry.staging_id}
                            className="flex items-start gap-3 rounded border border-primary/10 p-3"
                        >
                            <div className="flex-1 min-w-0">
                                <div className="font-medium text-sm">{entry.term}</div>
                                <div className="text-sm text-primary/60 mt-1">{entry.content}</div>
                                <div className="text-xs text-primary/40 mt-1">
                                    {entry.source_discourse_id && entry.source_topic_id && (
                                        <span>
                                            source: /t/{entry.source_discourse_id}/
                                            {entry.source_topic_id}
                                            {entry.source_post_number
                                                ? `#p-${entry.source_post_number}`
                                                : ''}
                                            {entry.link_reason ? ` — ${entry.link_reason}` : ''}
                                        </span>
                                    )}
                                    <span className="ml-2">
                                        <TimeAgo date={new Date(entry.created_at)} />
                                    </span>
                                </div>
                            </div>
                        </div>
                    ))}
                    {(!staging || staging.length === 0) && (
                        <div className="text-sm text-primary/40">No staged candidates.</div>
                    )}
                </div>
            </div>
            <div className="space-y-2">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Snapshots ({snapshots?.length ?? 0})
                </h2>
                {isLoadingSnapshots ? (
                    <div className="h-14 rounded bg-primary/5 animate-pulse" />
                ) : (
                    <div className="space-y-2">
                        {snapshots?.map((snapshot) => (
                            <div
                                key={snapshot.snapshot_id}
                                className="rounded border border-primary/20 p-3 space-y-1"
                            >
                                <div className="flex items-center gap-2 text-sm">
                                    <span className="font-medium">v{snapshot.version}</span>
                                    <span className="text-primary/40">
                                        <TimeAgo date={new Date(snapshot.created_at)} />
                                    </span>
                                </div>
                                {snapshot.summary && (
                                    <div className="text-sm text-primary/60">
                                        {snapshot.summary}
                                    </div>
                                )}
                            </div>
                        ))}
                        {(!snapshots || snapshots.length === 0) && (
                            <div className="text-sm text-primary/40">No snapshots yet.</div>
                        )}
                    </div>
                )}
            </div>
        </div>
    );
};

export const Route = createFileRoute('/admin/memory')({
    component: AdminMemoryPage,
});
