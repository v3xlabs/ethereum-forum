import { useMemo, useState } from 'react';
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';

import { useLlmUsage } from '@/api/admin';
import { formatTokens } from '@/util/format';

type UsageDatum = {
    date: string;
    summary: number;
    curator: number;
    digest: number;
    other: number;
    prompt: number;
    completion: number;
    total: number;
    runs: number;
    failures: number;
    byModel: Record<string, number>;
};

type StackMode = 'type' | 'direction' | 'model';

const typeSegments = [
    { key: 'summary', label: 'Summary', color: '#8b5cf6' },
    { key: 'curator', label: 'Curator', color: '#0d9488' },
    { key: 'digest', label: 'Digest', color: '#d97706' },
    { key: 'other', label: 'Other', color: '#2563eb' },
] as const;

const directionSegments = [
    { key: 'prompt', label: 'Prompt', color: '#8b5cf6' },
    { key: 'completion', label: 'Completion', color: '#0d9488' },
] as const;

const modelColorPalette = [
    '#8b5cf6',
    '#0d9488',
    '#d97706',
    '#2563eb',
    '#dc2626',
    '#65a30d',
    '#0891b2',
    '#c026d3',
    '#ea580c',
    '#4f46e5',
];

const TIMESCALES = [
    { days: 7, label: '7d' },
    { days: 30, label: '30d' },
    { days: 90, label: '90d' },
    { days: 180, label: '180d' },
] as const;

const emptyDatum = (date: string): UsageDatum => ({
    date,
    summary: 0,
    curator: 0,
    digest: 0,
    other: 0,
    prompt: 0,
    completion: 0,
    total: 0,
    runs: 0,
    failures: 0,
    byModel: {},
});

const utcDateString = (timestampMs: number) => new Date(timestampMs).toISOString().slice(0, 10);

const DAY_MS = 24 * 60 * 60 * 1000;

const buildSeries = (
    rows: {
        date: string;
        run_type: string;
        model_used: string;
        prompt_tokens: number;
        completion_tokens: number;
        total_tokens: number;
        runs: number;
        failures: number;
    }[],
    days: number
): UsageDatum[] => {
    const todayMs = Date.parse(`${utcDateString(Date.now())}T00:00:00Z`);
    let startMs = todayMs - (days - 1) * DAY_MS;

    for (const row of rows) {
        const rowMs = Date.parse(`${row.date}T00:00:00Z`);

        if (!Number.isNaN(rowMs) && rowMs < startMs) startMs = rowMs;
    }

    const byDate = new Map<string, UsageDatum>();

    for (let ms = startMs; ms <= todayMs; ms += DAY_MS) {
        const date = utcDateString(ms);

        byDate.set(date, emptyDatum(date));
    }

    for (const row of rows) {
        const datum = byDate.get(row.date);

        if (!datum) continue;

        if (row.run_type === 'summary' || row.run_type === 'curator' || row.run_type === 'digest') {
            datum[row.run_type] += row.total_tokens;
        } else {
            datum.other += row.total_tokens;
        }

        datum.prompt += row.prompt_tokens;
        datum.completion += row.completion_tokens;
        datum.total += row.total_tokens;
        datum.runs += row.runs;
        datum.failures += row.failures;

        const modelKey = row.model_used || 'unknown';

        datum.byModel[modelKey] = (datum.byModel[modelKey] ?? 0) + row.total_tokens;
    }

    return [...byDate.values()];
};

const shortModelName = (model: string) => model.split('/').pop() ?? model;

const shortDate = (date: string) => {
    const parsed = new Date(`${date}T00:00:00Z`);

    return parsed.toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
};

const projection = (series: UsageDatum[]) => {
    const firstActiveIndex = series.findIndex((datum) => datum.total > 0);

    if (firstActiveIndex === -1) return null;

    const daysSinceFirstActivity = series.length - firstActiveIndex;
    const windowDays = Math.min(7, daysSinceFirstActivity);
    const windowTotal = series
        .slice(series.length - windowDays)
        .reduce((sum, datum) => sum + datum.total, 0);
    const avgPerDay = windowTotal / windowDays;

    return { windowDays, avgPerDay, projectedMonthly: avgPerDay * 30 };
};

export const UsageChart = () => {
    const [days, setDays] = useState(30);
    const [mode, setMode] = useState<StackMode>('type');
    const { data: usage, isLoading, error } = useLlmUsage(days);

    const series = useMemo(
        () => (usage?.status === 'ok' ? buildSeries(usage.days, days) : []),
        [usage, days]
    );

    const modelSegments = useMemo(() => {
        const models = new Set<string>();

        for (const datum of series) {
            for (const model of Object.keys(datum.byModel)) {
                if (datum.byModel[model] > 0) models.add(model);
            }
        }

        return [...models].sort().map((model, i) => ({
            key: model,
            label: shortModelName(model),
            color: modelColorPalette[i % modelColorPalette.length],
            datumKey: model,
        }));
    }, [series]);

    const segments =
        mode === 'type' ? typeSegments : mode === 'direction' ? directionSegments : modelSegments;

    const getSegmentValue = (datum: UsageDatum, key: string): number => {
        if (mode === 'model') {
            return datum.byModel[key] ?? 0;
        }

        return datum[key as keyof UsageDatum] as number;
    };

    const activeSegments = segments.filter((segment) =>
        series.some((datum) => getSegmentValue(datum, segment.key) > 0)
    );
    const hasActivity = series.some((datum) => datum.total > 0);
    const projected = projection(series);

    if (isLoading) {
        return (
            <div className="space-y-2">
                <div className="h-5 w-40 rounded bg-primary/5 animate-pulse" />
                <div className="h-[260px] rounded bg-primary/5 animate-pulse" />
            </div>
        );
    }

    if (error) {
        return <div className="text-sm text-red-500">Failed to load usage: {String(error)}</div>;
    }

    if (usage?.status === 'unavailable') {
        return <div className="text-sm text-primary/40">Usage endpoint unavailable.</div>;
    }

    if (!hasActivity) {
        return (
            <div className="space-y-2">
                <div className="flex items-center justify-between gap-2 flex-wrap">
                    <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                        Token Usage ({days}d)
                    </h2>
                    <TimescaleSelector days={days} onChange={setDays} />
                </div>
                <div className="flex items-center justify-center h-[160px] rounded border border-primary/20 text-sm text-primary/40">
                    No usage recorded yet.
                </div>
            </div>
        );
    }

    return (
        <div className="space-y-3">
            <div className="flex items-center justify-between gap-2 flex-wrap">
                <h2 className="text-sm font-medium text-primary/60 uppercase tracking-wide">
                    Token Usage ({days}d)
                </h2>
                <div className="flex gap-2 flex-wrap">
                    <TimescaleSelector days={days} onChange={setDays} />
                    <div className="flex gap-1">
                        {(
                            [
                                ['type', 'By job type'],
                                ['direction', 'By direction'],
                                ['model', 'By model'],
                            ] as [StackMode, string][]
                        ).map(([value, label]) => (
                            <button
                                key={value}
                                className={`px-2.5 py-1 rounded text-xs transition-colors ${
                                    mode === value
                                        ? 'bg-primary/10 text-primary font-medium'
                                        : 'text-primary/60 hover:text-primary hover:bg-primary/5'
                                }`}
                                onClick={() => setMode(value)}
                            >
                                {label}
                            </button>
                        ))}
                    </div>
                </div>
            </div>
            <div className="h-[260px] text-primary">
                <ResponsiveContainer width="100%" height="100%">
                    <BarChart
                        data={series}
                        margin={{ top: 4, right: 0, bottom: 0, left: 0 }}
                        barCategoryGap="20%"
                    >
                        <XAxis
                            dataKey="date"
                            tickFormatter={shortDate}
                            tick={{ fill: 'currentColor', opacity: 0.45, fontSize: 11 }}
                            axisLine={false}
                            tickLine={false}
                            minTickGap={24}
                        />
                        <YAxis
                            tickFormatter={(value: number) => formatTokens(value)}
                            tick={{ fill: 'currentColor', opacity: 0.45, fontSize: 11 }}
                            axisLine={false}
                            tickLine={false}
                            width={44}
                        />
                        <Tooltip
                            cursor={{ fill: 'currentColor', opacity: 0.05 }}
                            content={({ active, label }) => {
                                if (!active || typeof label !== 'string') return null;

                                const datum = series.find((entry) => entry.date === label);

                                if (!datum) return null;

                                return (
                                    <div className="rounded border border-primary/20 bg-primary px-3 py-2 text-xs space-y-1 shadow-sm">
                                        <div className="font-medium">{shortDate(datum.date)}</div>
                                        {segments
                                            .filter(
                                                (segment) => getSegmentValue(datum, segment.key) > 0
                                            )
                                            .map((segment) => (
                                                <div
                                                    key={segment.key}
                                                    className="flex items-center gap-2"
                                                >
                                                    <span
                                                        className="w-2 h-2 rounded-full shrink-0"
                                                        style={{ backgroundColor: segment.color }}
                                                    />
                                                    <span className="text-primary/60">
                                                        {segment.label}
                                                    </span>
                                                    <span className="ml-auto font-medium tabular-nums">
                                                        {formatTokens(
                                                            getSegmentValue(datum, segment.key)
                                                        )}
                                                    </span>
                                                </div>
                                            ))}
                                        <div className="text-primary/40 pt-0.5">
                                            {datum.runs} run{datum.runs === 1 ? '' : 's'}
                                            {datum.failures > 0 && ` · ${datum.failures} failed`}
                                        </div>
                                    </div>
                                );
                            }}
                        />
                        {activeSegments.map((segment) => (
                            <Bar
                                key={`${mode}-${segment.key}`}
                                dataKey={(datum: UsageDatum) => getSegmentValue(datum, segment.key)}
                                stackId="tokens"
                                fill={segment.color}
                                maxBarSize={28}
                                isAnimationActive={false}
                            />
                        ))}
                    </BarChart>
                </ResponsiveContainer>
            </div>
            <div className="flex items-center gap-4 flex-wrap text-xs text-primary/60">
                {activeSegments.map((segment) => (
                    <div key={segment.key} className="flex items-center gap-1.5">
                        <span
                            className="w-2 h-2 rounded-full"
                            style={{ backgroundColor: segment.color }}
                        />
                        {segment.label}
                    </div>
                ))}
            </div>
            {projected && (
                <p className="text-xs text-primary/40">
                    Last {projected.windowDays} day{projected.windowDays === 1 ? '' : 's'} avg:{' '}
                    {formatTokens(Math.round(projected.avgPerDay))} tokens/day → ~
                    {formatTokens(Math.round(projected.projectedMonthly))}/month projected
                    {projected.windowDays < 7 && ' (limited data)'}
                </p>
            )}
        </div>
    );
};

const TimescaleSelector = ({
    days,
    onChange,
}: {
    days: number;
    onChange: (days: number) => void;
}) => (
    <div className="flex gap-1">
        {TIMESCALES.map(({ days: d, label }) => (
            <button
                key={d}
                className={`px-2.5 py-1 rounded text-xs transition-colors ${
                    days === d
                        ? 'bg-primary/10 text-primary font-medium'
                        : 'text-primary/60 hover:text-primary hover:bg-primary/5'
                }`}
                onClick={() => onChange(d)}
            >
                {label}
            </button>
        ))}
    </div>
);
