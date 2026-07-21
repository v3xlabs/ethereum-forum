import { useMutation, useQuery } from '@tanstack/react-query';

import { baseUrl } from './api';

const ADMIN_KEY = () => localStorage.getItem('admin_key');

const adminHeaders = (): Record<string, string> => {
    const key = ADMIN_KEY();
    const headers: Record<string, string> = {};

    if (key) {
        headers['X-Admin-Key'] = key;
    }

    return headers;
};

const adminFetch = async <T>(path: string, options?: RequestInit): Promise<T> => {
    const response = await fetch(new URL(path, baseUrl), {
        ...options,
        headers: {
            'Content-Type': 'application/json',
            ...adminHeaders(),
            ...options?.headers,
        },
    });

    if (response.status === 401) {
        throw new Error('Unauthorized: invalid or missing admin key');
    }

    if (!response.ok) {
        throw new Error(`Admin API error: ${response.statusText}`);
    }

    return response.json();
};

export interface AdminStats {
    database_topics: number;
    database_posts: number;
    meilisearch_documents: number | null;
}

export interface LlmRun {
    run_id: string;
    run_type: string;
    discourse_id: string | null;
    topic_id: number | null;
    prompt_tokens: number | null;
    completion_tokens: number | null;
    total_tokens: number | null;
    reasoning_tokens: number | null;
    model_used: string | null;
    tool_calls: number | null;
    tool_rounds: number | null;
    duration_ms: number | null;
    outcome: string;
    error: string | null;
    metadata: unknown | null;
    created_at: string;
    trace?: unknown;
}

export type LlmRunTraceEvent =
    | {
          type: 'tool_call';
          tool: string;
          label: string;
          status: 'ok' | 'error';
          detail?: string | null;
          output?: string | null;
          args?: unknown;
          at_ms: number;
          duration_ms: number;
      }
    | {
          type: 'completion';
          label: string;
          model?: string | null;
          prompt_tokens?: number | null;
          completion_tokens?: number | null;
          at_ms: number;
          duration_ms: number;
      }
    | { type: 'note'; label: string; at_ms: number };

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === 'object' && value !== null && !Array.isArray(value);

const asOptionalString = (value: unknown): string | null =>
    typeof value === 'string' ? value : null;

const asOptionalNumber = (value: unknown): number | null =>
    typeof value === 'number' ? value : null;

const parseTraceEvent = (raw: unknown): LlmRunTraceEvent | null => {
    if (!isRecord(raw)) return null;

    const { type, label, at_ms } = raw;

    if (typeof label !== 'string' || typeof at_ms !== 'number') return null;

    if (type === 'note') {
        return { type: 'note', label, at_ms };
    }

    const { duration_ms } = raw;

    if (typeof duration_ms !== 'number') return null;

    if (type === 'tool_call') {
        const { tool, status } = raw;

        if (typeof tool !== 'string') return null;

        if (status !== 'ok' && status !== 'error') return null;

        return {
            type: 'tool_call',
            tool,
            label,
            status,
            detail: asOptionalString(raw.detail),
            output: asOptionalString(raw.output),
            args: raw.args,
            at_ms,
            duration_ms,
        };
    }

    if (type === 'completion') {
        return {
            type: 'completion',
            label,
            model: asOptionalString(raw.model),
            prompt_tokens: asOptionalNumber(raw.prompt_tokens),
            completion_tokens: asOptionalNumber(raw.completion_tokens),
            at_ms,
            duration_ms,
        };
    }

    return null;
};

export const parseTrace = (trace: unknown): LlmRunTraceEvent[] | null => {
    if (!Array.isArray(trace)) return null;

    return trace
        .map(parseTraceEvent)
        .filter((event) => event !== null)
        .sort((a, b) => a.at_ms - b.at_ms);
};

export interface LlmMemoryEntry {
    entry_id: number;
    term: string;
    content: string;
    sources: unknown;
    updated_at: string;
}

export type MemoryLink = { url: string; reason?: string | null };

export const parseMemorySources = (raw: unknown): MemoryLink[] => {
    if (!Array.isArray(raw)) return [];

    return raw
        .map((item): MemoryLink | null => {
            if (!isRecord(item) || typeof item.url !== 'string' || item.url.length === 0) {
                return null;
            }

            return { url: item.url, reason: asOptionalString(item.reason) };
        })
        .filter((item) => item !== null);
};

export interface LlmMemorySnapshot {
    snapshot_id: number;
    version: number;
    memory_snapshot: unknown;
    curator_run_id: string | null;
    summary: string | null;
    created_at: string;
}

export interface LlmMemoryStaging {
    staging_id: number;
    term: string;
    content: string;
    source_discourse_id: string | null;
    source_topic_id: number | null;
    source_post_number: number | null;
    link_reason: string | null;
    created_at: string;
}

export interface LlmModelStats {
    model: string;
    runs: number;
    avg_prompt_tokens: number;
    avg_completion_tokens: number;
    avg_total_tokens: number;
    total_tokens: number;
}

export interface LlmModelStatsResponse {
    models: LlmModelStats[];
}

export interface AdminMetrics {
    total_runs: number;
    success_rate: number;
    avg_duration_ms: number;
    avg_total_tokens: number;
    total_tokens_all_time: number;
}

export interface CuratorTriggerResponse {
    success: boolean;
    message: string;
    output: {
        memory_updates: Array<{ term: string; content: string; sources: unknown }>;
        memory_removals?: unknown;
        snapshot_summary: string;
        action_log: string;
    } | null;
}

export type LlmUsageDay = {
    date: string;
    run_type: string;
    model_used: string;
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
    runs: number;
    failures: number;
};

export type LlmUsageResult = { status: 'ok'; days: LlmUsageDay[] } | { status: 'unavailable' };

const parseUsageDay = (raw: unknown): LlmUsageDay | null => {
    if (!isRecord(raw)) return null;

    const {
        date,
        run_type,
        model_used,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        runs,
        failures,
    } = raw;

    if (typeof date !== 'string' || typeof run_type !== 'string') return null;

    return {
        date,
        run_type,
        model_used: typeof model_used === 'string' ? model_used : 'unknown',
        prompt_tokens: asOptionalNumber(prompt_tokens) ?? 0,
        completion_tokens: asOptionalNumber(completion_tokens) ?? 0,
        total_tokens: asOptionalNumber(total_tokens) ?? 0,
        runs: asOptionalNumber(runs) ?? 0,
        failures: asOptionalNumber(failures) ?? 0,
    };
};

export const useAdminStats = () =>
    useQuery({
        queryKey: ['admin', 'stats'],
        queryFn: () => adminFetch<AdminStats>('admin/stats', { method: 'GET' }),
        enabled: !!ADMIN_KEY(),
    });

export const useAdminMetrics = () =>
    useQuery({
        queryKey: ['admin', 'metrics'],
        queryFn: () => adminFetch<AdminMetrics>('admin/llm/metrics', { method: 'GET' }),
        enabled: !!ADMIN_KEY(),
    });

export const useSystemPrompts = () =>
    useQuery({
        queryKey: ['admin', 'system-prompt'],
        queryFn: () =>
            adminFetch<{ summary_prompt: string; digest_prompt: string; curator_prompt: string }>(
                'admin/llm/system-prompt',
                { method: 'GET' }
            ),
        enabled: !!ADMIN_KEY(),
    });

export const useLlmRuns = (runType?: string) =>
    useQuery({
        queryKey: ['admin', 'runs', runType],
        queryFn: () => {
            const params = new URLSearchParams();

            if (runType) params.set('run_type', runType);

            params.set('limit', '50');

            return adminFetch<LlmRun[]>(`admin/llm/runs?${params}`, { method: 'GET' });
        },
        enabled: !!ADMIN_KEY(),
        refetchInterval: (query) => {
            const { data } = query.state;

            return data?.some((r) => r.outcome === 'running') ? 3000 : false;
        },
        staleTime: 1000,
    });

export const useLlmRunDetail = (runId: string | null) =>
    useQuery({
        queryKey: ['admin', 'runs', 'detail', runId],
        queryFn: () => {
            if (!runId) throw new Error('Missing run id');

            return adminFetch<LlmRun>(`admin/llm/runs/${runId}`, { method: 'GET' });
        },
        enabled: !!ADMIN_KEY() && !!runId,
    });

export const useLlmUsage = (days = 30) =>
    useQuery({
        queryKey: ['admin', 'usage', days],
        queryFn: async (): Promise<LlmUsageResult> => {
            const response = await fetch(new URL(`admin/llm/usage?days=${days}`, baseUrl), {
                method: 'GET',
                headers: adminHeaders(),
            });

            if (response.status === 404) return { status: 'unavailable' };

            if (response.status === 401) {
                throw new Error('Unauthorized: invalid or missing admin key');
            }

            if (!response.ok) {
                throw new Error(`Admin API error: ${response.statusText}`);
            }

            const body: unknown = await response.json();
            const rawDays = isRecord(body) && Array.isArray(body.days) ? body.days : [];

            return {
                status: 'ok',
                days: rawDays.map(parseUsageDay).filter((day) => day !== null),
            };
        },
        enabled: !!ADMIN_KEY(),
    });

export const useLlmPerModelStats = (days = 30) =>
    useQuery({
        queryKey: ['admin', 'per-model', days],
        queryFn: () =>
            adminFetch<LlmModelStatsResponse>(`admin/llm/per-model?days=${days}`, {
                method: 'GET',
            }),
        enabled: !!ADMIN_KEY(),
    });

export const useMemory = () =>
    useQuery({
        queryKey: ['admin', 'memory'],
        queryFn: () => adminFetch<LlmMemoryEntry[]>('admin/llm/memory', { method: 'GET' }),
        enabled: !!ADMIN_KEY(),
    });

export const useSnapshots = () =>
    useQuery({
        queryKey: ['admin', 'snapshots'],
        queryFn: () => adminFetch<LlmMemorySnapshot[]>('admin/llm/snapshots', { method: 'GET' }),
        enabled: !!ADMIN_KEY(),
    });

export const useStaging = (limit = 100) =>
    useQuery({
        queryKey: ['admin', 'staging', limit],
        queryFn: () =>
            adminFetch<LlmMemoryStaging[]>(`admin/llm/memory/staging?limit=${limit}`, {
                method: 'GET',
            }),
        enabled: !!ADMIN_KEY(),
    });

export const useTriggerCurator = () =>
    useMutation({
        mutationFn: () =>
            adminFetch<CuratorTriggerResponse>('admin/llm/curator/trigger', {
                method: 'POST',
            }),
    });

export const useDeleteMemory = () =>
    useMutation({
        mutationFn: (entryId: number) =>
            adminFetch<void>(`admin/llm/memory/${entryId}`, { method: 'DELETE' }),
    });

export const useUpsertMemory = () =>
    useMutation({
        mutationFn: (entry: { term: string; content: string; sources?: MemoryLink[] }) =>
            adminFetch<LlmMemoryEntry>('admin/llm/memory', {
                method: 'POST',
                body: JSON.stringify(entry),
            }),
    });

export const useForceDigest = () =>
    useMutation({
        mutationFn: () => adminFetch<{ digest_id: number }>('admin/digest', { method: 'POST' }),
    });

export const useDeleteDigest = () =>
    useMutation({
        mutationFn: () => adminFetch<void>('admin/digest', { method: 'DELETE' }),
    });
