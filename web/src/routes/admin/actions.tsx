import { useQuery, useQueryClient } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { LuRefreshCw, LuTrash2 } from 'react-icons/lu';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import {
    parseMemorySources,
    useDeleteDigest,
    useForceDigest,
    useLlmRuns,
    useSnapshots,
    useTriggerCurator,
} from '@/api/admin';
import { baseUrl } from '@/api/api';
import { MemorySourceLinks } from '@/components/admin/MemorySourceLinks';
import { formatDurationMs, formatTokens } from '@/util/format';

const memoryRemovals = (raw: unknown): string[] =>
    Array.isArray(raw) ? raw.filter((item) => typeof item === 'string') : [];

type LatestDigest = { digest_text: string | null; created_at: string };

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === 'object' && value !== null && !Array.isArray(value);

const parseLatestDigest = (raw: unknown): LatestDigest | null => {
    if (!isRecord(raw) || typeof raw.created_at !== 'string') return null;

    return {
        digest_text: typeof raw.digest_text === 'string' ? raw.digest_text : null,
        created_at: raw.created_at,
    };
};

const useLatestDigest = () =>
    useQuery({
        queryKey: ['admin', 'digest'],
        queryFn: async (): Promise<LatestDigest | null> => {
            const response = await fetch(new URL('digest', baseUrl));

            if (!response.ok) return null;

            const body: unknown = await response.json();

            return parseLatestDigest(body);
        },
        enabled: !!localStorage.getItem('admin_key'),
    });

const DigestSection = () => {
    const forceDigest = useForceDigest();
    const deleteDigest = useDeleteDigest();
    const queryClient = useQueryClient();
    const { data: latestDigest } = useLatestDigest();

    const invalidate = () => {
        queryClient.invalidateQueries({ queryKey: ['admin', 'runs'] });
        queryClient.invalidateQueries({ queryKey: ['admin', 'digest'] });
    };

    return (
        <div className="space-y-4">
            <div className="flex items-center justify-between gap-2 flex-wrap">
                <h2 className="text-lg font-medium">Digest</h2>
                <div className="flex gap-2">
                    <button
                        className="flex items-center gap-2 px-3 py-2 bg-green-500/10 text-green-500 rounded text-sm hover:bg-green-500/20 transition-colors disabled:opacity-50"
                        onClick={() => forceDigest.mutate(undefined, { onSuccess: invalidate })}
                        disabled={forceDigest.isPending}
                    >
                        <LuRefreshCw
                            className={`w-4 h-4 ${forceDigest.isPending ? 'animate-spin' : ''}`}
                        />
                        Force Digest
                    </button>
                    <button
                        className="flex items-center gap-2 px-3 py-2 bg-red-500/10 text-red-500 rounded text-sm hover:bg-red-500/20 transition-colors disabled:opacity-50"
                        onClick={() => deleteDigest.mutate(undefined, { onSuccess: invalidate })}
                        disabled={deleteDigest.isPending}
                    >
                        <LuTrash2 className="w-4 h-4" />
                        Delete & Regenerate
                    </button>
                </div>
            </div>
            {forceDigest.data && (
                <div className="rounded bg-green-500/10 border border-green-500/20 p-3 text-sm text-green-600">
                    Digest generated successfully
                </div>
            )}
            {deleteDigest.isSuccess && (
                <div className="rounded bg-green-500/10 border border-green-500/20 p-3 text-sm text-green-600">
                    Digest deleted
                </div>
            )}
            {latestDigest ? (
                <div className="rounded border border-primary/20 p-4 space-y-2">
                    <h3 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                        Latest Digest
                    </h3>
                    <div className="prose prose-sm max-w-none text-sm">
                        <Markdown remarkPlugins={[remarkGfm]}>
                            {latestDigest.digest_text ?? 'No content'}
                        </Markdown>
                    </div>
                    <p className="text-xs text-primary/40">
                        {new Date(latestDigest.created_at).toLocaleString()}
                    </p>
                </div>
            ) : (
                <p className="text-sm text-primary/40">No digest has been generated yet.</p>
            )}
        </div>
    );
};

const CuratorSection = () => {
    const curatorRuns = useLlmRuns('curator');
    const { data: snapshots } = useSnapshots();
    const triggerMutation = useTriggerCurator();
    const queryClient = useQueryClient();

    const lastRun = curatorRuns.data?.[0];
    const lastSnapshot = snapshots?.[0];

    const handleTrigger = () => {
        triggerMutation.mutate(undefined, {
            onSuccess: () => {
                queryClient.invalidateQueries({ queryKey: ['admin', 'runs'] });
                queryClient.invalidateQueries({ queryKey: ['admin', 'snapshots'] });
                queryClient.invalidateQueries({ queryKey: ['admin', 'memory'] });
            },
        });
    };

    return (
        <div className="space-y-4">
            <div className="flex items-center justify-between gap-2 flex-wrap">
                <h2 className="text-lg font-medium">Curator</h2>
                <button
                    className="flex items-center gap-2 px-3 py-2 bg-primary/10 rounded text-sm hover:bg-primary/20 transition-colors disabled:opacity-50"
                    onClick={handleTrigger}
                    disabled={triggerMutation.isPending}
                >
                    <LuRefreshCw
                        className={`w-4 h-4 ${triggerMutation.isPending ? 'animate-spin' : ''}`}
                    />
                    Run Curator
                </button>
            </div>
            {triggerMutation.data && (
                <div
                    className={`rounded p-4 text-sm ${
                        triggerMutation.data.success
                            ? 'bg-green-500/10 border border-green-500/20'
                            : 'bg-red-500/10 border border-red-500/20'
                    }`}
                >
                    <div className="font-medium mb-1">
                        {triggerMutation.data.success ? 'Success' : 'Failed'}
                    </div>
                    <div className="text-primary/60">{triggerMutation.data.message}</div>
                    {triggerMutation.data.output && (
                        <div className="mt-2 space-y-1 text-xs text-primary/60">
                            <div>
                                Memory updates: {triggerMutation.data.output.memory_updates.length}
                            </div>
                            <div>
                                Snapshot summary: {triggerMutation.data.output.snapshot_summary}
                            </div>
                            <div>Action log: {triggerMutation.data.output.action_log}</div>
                            {memoryRemovals(triggerMutation.data.output.memory_removals).length >
                                0 && (
                                <div>
                                    Memory removals:{' '}
                                    {memoryRemovals(
                                        triggerMutation.data.output.memory_removals
                                    ).join(', ')}
                                </div>
                            )}
                            {triggerMutation.data.output.memory_updates.length > 0 && (
                                <div className="mt-2 space-y-1">
                                    {triggerMutation.data.output.memory_updates.map(
                                        (update, index) => (
                                            <div key={index} className="p-2 bg-primary/5 rounded">
                                                <strong>{update.term}</strong>: {update.content}
                                                <MemorySourceLinks
                                                    sources={parseMemorySources(update.sources)}
                                                />
                                            </div>
                                        )
                                    )}
                                </div>
                            )}
                        </div>
                    )}
                </div>
            )}
            {lastRun && (
                <div className="rounded border border-primary/20 p-4 space-y-2">
                    <h3 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                        Last Curator Run
                    </h3>
                    <div className="grid grid-cols-2 gap-2 text-sm">
                        <div>
                            <span className="text-primary/60">Time:</span>{' '}
                            {new Date(lastRun.created_at).toLocaleString()}
                        </div>
                        <div>
                            <span className="text-primary/60">Outcome:</span>{' '}
                            <span
                                className={
                                    lastRun.outcome === 'success'
                                        ? 'text-green-500'
                                        : 'text-red-500'
                                }
                            >
                                {lastRun.outcome}
                            </span>
                        </div>
                        <div>
                            <span className="text-primary/60">Tokens:</span>{' '}
                            <span
                                title={
                                    lastRun.total_tokens !== null
                                        ? `${lastRun.total_tokens} tokens`
                                        : undefined
                                }
                            >
                                {formatTokens(lastRun.total_tokens)}
                            </span>
                        </div>
                        <div>
                            <span className="text-primary/60">Duration:</span>{' '}
                            {formatDurationMs(lastRun.duration_ms)}
                        </div>
                    </div>
                </div>
            )}
            {lastSnapshot && (
                <div className="rounded border border-primary/20 p-4 space-y-2">
                    <h3 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                        Latest Snapshot (v{lastSnapshot.version})
                    </h3>
                    {lastSnapshot.summary && (
                        <div className="text-sm text-primary/80">{lastSnapshot.summary}</div>
                    )}
                    <div className="text-xs text-primary/40">
                        {new Date(lastSnapshot.created_at).toLocaleString()}
                    </div>
                </div>
            )}
        </div>
    );
};

const AdminActionsPage = () => (
    <div className="space-y-10">
        <h1 className="text-2xl font-semibold">Actions</h1>
        <CuratorSection />
        <DigestSection />
    </div>
);

export const Route = createFileRoute('/admin/actions')({
    component: AdminActionsPage,
});
