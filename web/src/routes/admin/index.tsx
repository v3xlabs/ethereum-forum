import { createFileRoute, Link } from '@tanstack/react-router';
import { ReactNode, useState } from 'react';
import {
    LuActivity,
    LuBook,
    LuClock,
    LuDatabase,
    LuFileText,
    LuHistory,
    LuTerminal,
    LuTriangle,
    LuZap,
} from 'react-icons/lu';

import { useAdminMetrics, useAdminStats, useLlmPerModelStats } from '@/api/admin';
import { UsageChart } from '@/components/admin/UsageChart';
import { formatDurationMs, formatTokens } from '@/util/format';

const StatCard = ({
    icon,
    label,
    value,
}: {
    icon: ReactNode;
    label: string;
    value: string | number;
}) => (
    <div className="rounded border border-primary/20 p-4 space-y-1">
        <div className="flex items-center gap-2 text-primary/60 text-sm">
            {icon}
            <span>{label}</span>
        </div>
        <div className="text-2xl font-semibold">{value}</div>
    </div>
);

const quickLinks = [
    {
        to: '/admin/runs',
        label: 'Run History',
        description: 'Per-run tokens, durations, and traces',
        icon: <LuHistory />,
    },
    {
        to: '/admin/memory',
        label: 'Memory',
        description: 'Shared glossary entries and snapshots',
        icon: <LuBook />,
    },
    {
        to: '/admin/prompts',
        label: 'Prompts',
        description: 'System prompts injected into runs',
        icon: <LuTerminal />,
    },
    {
        to: '/admin/actions',
        label: 'Actions',
        description: 'Trigger the curator or digest',
        icon: <LuZap />,
    },
] as const;

const PER_MODEL_TIMESCALES = [
    { days: 7, label: '7d' },
    { days: 30, label: '30d' },
    { days: 90, label: '90d' },
    { days: 365, label: 'All' },
] as const;

const shortModelName = (model: string) => model.split('/').pop() ?? model;

const PerModelStatsTable = () => {
    const [days, setDays] = useState(30);
    const { data, isLoading, error } = useLlmPerModelStats(days);

    return (
        <div className="space-y-3">
            <div className="flex items-center justify-between gap-2 flex-wrap">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Avg tokens per run by model
                </h2>
                <div className="flex gap-1">
                    {PER_MODEL_TIMESCALES.map(({ days: d, label }) => (
                        <button
                            key={d}
                            className={`px-2.5 py-1 rounded text-xs transition-colors ${
                                days === d
                                    ? 'bg-primary/10 text-primary font-medium'
                                    : 'text-primary/60 hover:text-primary hover:bg-primary/5'
                            }`}
                            onClick={() => setDays(d)}
                        >
                            {label}
                        </button>
                    ))}
                </div>
            </div>
            {isLoading && <div className="h-20 rounded bg-primary/5 animate-pulse" />}
            {error && <div className="text-sm text-red-500">Failed to load: {String(error)}</div>}
            {data && data.models.length === 0 && (
                <div className="text-sm text-primary/40 py-4 text-center">
                    No completed runs in this period.
                </div>
            )}
            {data && data.models.length > 0 && (
                <div className="overflow-x-auto">
                    <table className="w-full text-sm">
                        <thead>
                            <tr className="border-b border-primary/20">
                                <th className="text-left py-2 px-3 text-primary/60 font-medium">
                                    Model
                                </th>
                                <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                    Runs
                                </th>
                                <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                    Avg prompt
                                </th>
                                <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                    Avg completion
                                </th>
                                <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                    Avg total
                                </th>
                                <th className="text-right py-2 px-3 text-primary/60 font-medium">
                                    Total tokens
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {data.models.map((row) => (
                                <tr
                                    key={row.model}
                                    className="border-b border-primary/10 hover:bg-primary/5"
                                >
                                    <td className="py-2 px-3 font-medium">
                                        {shortModelName(row.model)}
                                        <span className="block text-xs text-primary/40">
                                            {row.model}
                                        </span>
                                    </td>
                                    <td className="py-2 px-3 text-right tabular-nums">
                                        {row.runs}
                                    </td>
                                    <td className="py-2 px-3 text-right tabular-nums">
                                        {formatTokens(Math.round(row.avg_prompt_tokens))}
                                    </td>
                                    <td className="py-2 px-3 text-right tabular-nums">
                                        {formatTokens(Math.round(row.avg_completion_tokens))}
                                    </td>
                                    <td className="py-2 px-3 text-right tabular-nums">
                                        {formatTokens(Math.round(row.avg_total_tokens))}
                                    </td>
                                    <td className="py-2 px-3 text-right tabular-nums">
                                        {formatTokens(row.total_tokens)}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </div>
    );
};

const AdminOverviewPage = () => {
    const { data: stats } = useAdminStats();
    const { data: metrics } = useAdminMetrics();

    return (
        <div className="space-y-6">
            <h1 className="text-2xl font-semibold">Admin Dashboard</h1>
            <div className="grid grid-cols-2 lg:grid-cols-3 gap-4">
                <StatCard
                    icon={<LuDatabase />}
                    label="Topics"
                    value={stats?.database_topics ?? '—'}
                />
                <StatCard
                    icon={<LuFileText />}
                    label="Posts"
                    value={stats?.database_posts ?? '—'}
                />
                <StatCard
                    icon={<LuActivity />}
                    label="Total Runs"
                    value={metrics?.total_runs ?? '—'}
                />
                <StatCard
                    icon={<LuTriangle />}
                    label="Success Rate"
                    value={metrics ? `${metrics.success_rate.toFixed(1)}%` : '—'}
                />
                <StatCard
                    icon={<LuClock />}
                    label="Avg Duration"
                    value={metrics ? formatDurationMs(metrics.avg_duration_ms) : '—'}
                />
                <StatCard
                    icon={<LuFileText />}
                    label="Avg Tokens"
                    value={metrics ? formatTokens(Math.round(metrics.avg_total_tokens)) : '—'}
                />
            </div>
            <UsageChart />
            <PerModelStatsTable />
            <div className="space-y-2">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Sections
                </h2>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                    {quickLinks.map(({ to, label, description, icon }) => (
                        <Link
                            key={to}
                            to={to}
                            className="rounded border border-primary/20 p-4 space-y-1 hover:bg-primary/5 transition-colors"
                        >
                            <div className="flex items-center gap-2 text-sm font-medium">
                                {icon}
                                <span>{label}</span>
                            </div>
                            <div className="text-sm text-primary/60">{description}</div>
                        </Link>
                    ))}
                </div>
            </div>
        </div>
    );
};

export const Route = createFileRoute('/admin/')({
    component: AdminOverviewPage,
});
