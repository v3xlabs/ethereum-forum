import { createFileRoute } from '@tanstack/react-router';
import { Fragment, useState } from 'react';
import {
    LuCheck,
    LuChevronDown,
    LuChevronRight,
    LuCpu,
    LuInfo,
    LuLoader,
    LuTriangle,
    LuX,
} from 'react-icons/lu';

import { LlmRun, LlmRunTraceEvent, parseTrace, useLlmRunDetail, useLlmRuns } from '@/api/admin';
import { TimeAgo } from '@/components/TimeAgo';
import { formatDurationMs, formatTokens } from '@/util/format';

const TraceEventRow = ({ event }: { event: LlmRunTraceEvent }) => {
    if (event.type === 'note') {
        return (
            <div className="flex items-center gap-2 py-1 text-primary/40">
                <LuInfo className="w-3.5 h-3.5 shrink-0" />
                <span>{event.label}</span>
                <span className="ml-auto tabular-nums" title={`at ${event.at_ms}ms`}>
                    +{formatDurationMs(event.at_ms)}
                </span>
            </div>
        );
    }

    if (event.type === 'completion') {
        return (
            <div className="flex items-center gap-2 py-1 flex-wrap">
                <LuCpu className="w-3.5 h-3.5 shrink-0 text-primary/60" />
                <span className="font-medium">{event.label}</span>
                {event.model && <span className="text-primary/40">{event.model}</span>}
                <span className="text-primary/60 tabular-nums">
                    in {formatTokens(event.prompt_tokens)} / out{' '}
                    {formatTokens(event.completion_tokens)}
                </span>
                <span
                    className="ml-auto text-primary/40 tabular-nums"
                    title={`at ${event.at_ms}ms`}
                >
                    {formatDurationMs(event.duration_ms)}
                </span>
            </div>
        );
    }

    return (
        <div className="py-1 space-y-0.5">
            <div className="flex items-center gap-2">
                {event.status === 'ok' ? (
                    <LuCheck className="w-3.5 h-3.5 shrink-0 text-green-500" />
                ) : (
                    <LuX className="w-3.5 h-3.5 shrink-0 text-red-500" />
                )}
                <span className="font-medium">{event.label}</span>
                <span className="text-primary/40">{event.tool}</span>
                <span
                    className="ml-auto text-primary/40 tabular-nums"
                    title={`at ${event.at_ms}ms`}
                >
                    {formatDurationMs(event.duration_ms)}
                </span>
            </div>
            {event.detail && <div className="ml-5 text-primary/60">{event.detail}</div>}
            {event.args !== undefined && event.args !== null && (
                <details className="ml-5">
                    <summary className="text-xs text-primary/40 cursor-pointer select-none">
                        args
                    </summary>
                    <pre className="mt-1 p-2 rounded bg-primary/5 text-xs overflow-x-auto whitespace-pre-wrap max-h-40 overflow-y-auto">
                        {JSON.stringify(event.args, null, 2)}
                    </pre>
                </details>
            )}
            {event.output && (
                <details className="ml-5">
                    <summary className="text-xs text-primary/40 cursor-pointer select-none">
                        output
                    </summary>
                    <pre className="mt-1 p-2 rounded bg-primary/5 text-xs overflow-x-auto whitespace-pre-wrap max-h-60 overflow-y-auto">
                        {event.output}
                    </pre>
                </details>
            )}
        </div>
    );
};

const RunDetail = ({ runId }: { runId: string }) => {
    const { data: run, isLoading, error } = useLlmRunDetail(runId);

    if (isLoading) {
        return (
            <div className="space-y-2 py-1">
                <div className="h-4 w-64 rounded bg-primary/5 animate-pulse" />
                <div className="h-4 w-96 rounded bg-primary/5 animate-pulse" />
                <div className="h-4 w-48 rounded bg-primary/5 animate-pulse" />
            </div>
        );
    }

    if (error) {
        return <div className="text-sm text-red-500 py-1">Failed to load run: {String(error)}</div>;
    }

    if (!run) return null;

    const trace = parseTrace(run.trace);

    return (
        <div className="space-y-4 py-2 text-sm">
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-x-6 gap-y-1">
                <div>
                    <span className="text-primary/60">Run ID:</span>{' '}
                    <code className="text-xs bg-primary/5 px-1 rounded">{run.run_id}</code>
                </div>
                <div>
                    <span className="text-primary/60">Model:</span> {run.model_used ?? '—'}
                </div>
                <div>
                    <span className="text-primary/60">Prompt / completion:</span>{' '}
                    <span title={`${run.prompt_tokens ?? '?'} / ${run.completion_tokens ?? '?'}`}>
                        {formatTokens(run.prompt_tokens)} / {formatTokens(run.completion_tokens)}
                    </span>
                </div>
                <div>
                    <span className="text-primary/60">Tools:</span> {run.tool_calls ?? '—'} calls,{' '}
                    {run.tool_rounds ?? '—'} rounds
                </div>
            </div>
            {run.error && (
                <div className="space-y-1">
                    <div className="font-medium text-red-500">Error</div>
                    <pre className="p-3 rounded bg-red-500/5 border border-red-500/20 text-red-400 overflow-x-auto whitespace-pre-wrap">
                        {run.error}
                    </pre>
                </div>
            )}
            <div className="space-y-1">
                <div className="font-medium text-primary/60">Trace</div>
                {trace === null || trace.length === 0 ? (
                    <div className="text-primary/40">No trace recorded for this run.</div>
                ) : (
                    <div className="divide-y divide-primary/10 text-xs">
                        {trace.map((event, index) => (
                            <TraceEventRow key={index} event={event} />
                        ))}
                    </div>
                )}
            </div>
            {run.metadata !== null && run.metadata !== undefined && (
                <div className="space-y-1">
                    <div className="font-medium text-primary/60">Metadata</div>
                    <pre className="p-2 rounded bg-primary/5 border border-primary/20 text-xs overflow-x-auto whitespace-pre-wrap max-h-60 overflow-y-auto">
                        {JSON.stringify(run.metadata, null, 2)}
                    </pre>
                </div>
            )}
        </div>
    );
};

const outcomeClass = (outcome: string) => {
    if (outcome === 'success') return 'text-green-500';

    if (outcome === 'failure') return 'text-red-500';

    if (outcome === 'running') return 'text-yellow-500';

    return 'text-yellow-500';
};

const AdminRunsPage = () => {
    const [filter, setFilter] = useState('');
    const [expandedRunId, setExpandedRunId] = useState<string | null>(null);
    const { data: runs, isLoading, error } = useLlmRuns(filter || undefined);

    return (
        <div className="space-y-4">
            <h1 className="text-2xl font-semibold">Run History</h1>
            <div className="flex gap-2">
                {['', 'summary', 'digest', 'curator'].map((type) => (
                    <button
                        key={type}
                        className={`px-3 py-1.5 rounded text-sm capitalize transition-colors ${
                            filter === type
                                ? 'bg-primary/20 text-primary'
                                : 'bg-primary/5 text-primary/60 hover:bg-primary/10'
                        }`}
                        onClick={() => {
                            setFilter(type);
                            setExpandedRunId(null);
                        }}
                    >
                        {type || 'All'}
                    </button>
                ))}
            </div>
            {isLoading && <div className="text-primary/60">Loading...</div>}
            {error && <div className="text-red-500">Error: {String(error)}</div>}
            <div className="overflow-x-auto">
                <table className="w-full text-sm">
                    <thead>
                        <tr className="border-b border-primary/20">
                            <th className="w-6" />
                            <th className="text-left py-2 px-3 text-primary/60 font-medium">
                                Type
                            </th>
                            <th className="text-left py-2 px-3 text-primary/60 font-medium">
                                Topic
                            </th>
                            <th className="text-left py-2 px-3 text-primary/60 font-medium">
                                Model
                            </th>
                            <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                Tokens
                            </th>
                            <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                Duration
                            </th>
                            <th className="text-left py-2 px-3 text-primary/60 font-medium">
                                Outcome
                            </th>
                            <th className="text-left py-2 px-3 text-primary/60 font-medium">
                                Created
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {runs?.map((run: LlmRun) => {
                            const isExpanded = expandedRunId === run.run_id;

                            return (
                                <Fragment key={run.run_id}>
                                    <tr
                                        className={`border-b border-primary/10 hover:bg-primary/5 cursor-pointer transition-colors ${isExpanded ? 'bg-primary/5' : ''}`}
                                        onClick={() =>
                                            setExpandedRunId(isExpanded ? null : run.run_id)
                                        }
                                    >
                                        <td className="pl-3 text-primary/40">
                                            {isExpanded ? (
                                                <LuChevronDown className="w-3.5 h-3.5" />
                                            ) : (
                                                <LuChevronRight className="w-3.5 h-3.5" />
                                            )}
                                        </td>
                                        <td className="py-2 px-3 capitalize">{run.run_type}</td>
                                        <td className="py-2 px-3 text-primary/60">
                                            {run.topic_id !== null
                                                ? `${run.discourse_id ? `${run.discourse_id}/` : ''}#${run.topic_id}`
                                                : '—'}
                                        </td>
                                        <td className="py-2 px-3 text-primary/60">
                                            {run.model_used ? run.model_used.split('/').pop() : '—'}
                                        </td>
                                        <td
                                            className="py-2 px-3 text-right tabular-nums"
                                            title={
                                                run.total_tokens !== null
                                                    ? `${run.total_tokens} tokens`
                                                    : undefined
                                            }
                                        >
                                            {formatTokens(run.total_tokens)}
                                        </td>
                                        <td className="py-2 px-3 text-right tabular-nums">
                                            {formatDurationMs(run.duration_ms)}
                                        </td>
                                        <td className="py-2 px-3">
                                            <span
                                                className={`inline-flex items-center gap-1 ${outcomeClass(run.outcome)}`}
                                            >
                                                {run.outcome === 'running' && (
                                                    <LuLoader className="w-3.5 h-3.5 animate-spin" />
                                                )}
                                                {run.outcome}
                                                {run.error && run.outcome !== 'running' && (
                                                    <LuTriangle
                                                        className="w-3.5 h-3.5"
                                                        title={run.error}
                                                    />
                                                )}
                                            </span>
                                        </td>
                                        <td className="py-2 px-3 text-primary/40 whitespace-nowrap">
                                            <TimeAgo date={new Date(run.created_at)} />
                                        </td>
                                    </tr>
                                    {isExpanded && (
                                        <tr className="border-b border-primary/10">
                                            <td colSpan={8} className="px-3 bg-primary/[0.02]">
                                                <RunDetail runId={run.run_id} />
                                            </td>
                                        </tr>
                                    )}
                                </Fragment>
                            );
                        })}
                    </tbody>
                </table>
                {(!runs || runs.length === 0) && !isLoading && (
                    <div className="text-sm text-primary/40 py-4 text-center">No runs found.</div>
                )}
            </div>
        </div>
    );
};

export const Route = createFileRoute('/admin/runs')({
    component: AdminRunsPage,
});
