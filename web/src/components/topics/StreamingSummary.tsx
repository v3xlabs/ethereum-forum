import { Link } from '@tanstack/react-router';
import React from 'react';
import {
    LuBookOpen,
    LuBrain,
    LuCheck,
    LuFileText,
    LuList,
    LuLoader,
    LuMessageCircle,
    LuNotebookPen,
    LuRefreshCw,
    LuSearch,
    LuSparkles,
    LuTriangleAlert,
    LuUsers,
} from 'react-icons/lu';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import {
    SummaryActivity,
    useCachedTopicSummary,
    useStartTopicSummaryStream,
    useTopicSummaryStream,
} from '@/api/topics';
import { queryClient } from '@/util/query';

interface StreamingSummaryProps {
    discourseId: string;
    topicId: number;
    /** 'generate' starts/attaches a run on mount; 'cached' renders the stored summary only. */
    mode?: 'generate' | 'cached';
    initialTab?: SummaryTabId;
}

interface PerspectivePerson {
    username?: string;
    summary?: string;
}

export interface PerspectiveGroup {
    label?: string;
    people?: PerspectivePerson[];
}

export interface ParsedSummary {
    overview?: string;
    key_points?: string[];
    open_questions?: string[];
    perspectives?: PerspectiveGroup[];
    changelog_entry?: {
        period_start?: string;
        period_end?: string;
        post_range?: number[];
        entry?: string;
    };
}

const stripCodeFences = (text: string): string => {
    const t = text.trim();

    if (t.startsWith('```')) {
        const nl = t.indexOf('\n');

        if (nl === -1) return t;

        const inner = t.slice(nl + 1);

        return inner.endsWith('```') ? inner.slice(0, -3).trim() : inner;
    }

    return t;
};

export const tryParseSummary = (text: string): ParsedSummary | null => {
    if (!text.trim()) return null;

    const clean = stripCodeFences(text);

    try {
        const parsed = JSON.parse(clean);

        if (parsed && typeof parsed === 'object' && (parsed.overview || parsed.key_points)) {
            return parsed as ParsedSummary;
        }

        return null;
    } catch {
        return null;
    }
};

// Decodes the inside of a (possibly truncated) JSON string literal.
const decodeJsonStringBody = (body: string): string => {
    let out = '';

    for (let i = 0; i < body.length; i++) {
        const ch = body[i];

        if (ch !== '\\') {
            out += ch;
            continue;
        }

        const next = body[i + 1];

        if (next === undefined) break;

        i++;

        if (next === 'n') out += '\n';
        else if (next === 't') out += '\t';
        else if (next === 'r') out += '';
        else if (next === 'u') {
            const hex = body.slice(i + 1, i + 5);

            if (/^[0-9a-fA-F]{4}$/.test(hex)) {
                out += String.fromCharCode(Number.parseInt(hex, 16));
                i += 4;
            }
        } else out += next;
    }

    return out;
};

const findStringFieldStart = (raw: string, field: string): number => {
    const marker = `"${field}"`;
    const markerIndex = raw.indexOf(marker);

    if (markerIndex === -1) return -1;

    let i = markerIndex + marker.length;

    while (i < raw.length && /\s/.test(raw[i])) i++;

    if (raw[i] !== ':') return -1;

    i++;

    while (i < raw.length && /\s/.test(raw[i])) i++;

    if (raw[i] !== '"') return -1;

    return i + 1;
};

// Extracts a string field from mid-stream JSON so the overview can render
// while the model is still writing it.
const extractStringField = (raw: string, field: string): string | null => {
    const start = findStringFieldStart(raw, field);

    if (start === -1) return null;

    let end = -1;

    for (let i = start; i < raw.length; i++) {
        if (raw[i] === '\\') {
            i++;
            continue;
        }

        if (raw[i] === '"') {
            end = i;
            break;
        }
    }

    const body = end === -1 ? raw.slice(start) : raw.slice(start, end);

    return decodeJsonStringBody(body);
};

// Returns the fully-received items of a string array field in mid-stream JSON.
const extractCompletedStrings = (raw: string, field: string): string[] => {
    const marker = `"${field}"`;
    const fieldIndex = raw.indexOf(marker);

    if (fieldIndex === -1) return [];

    const bracketIndex = raw.indexOf('[', fieldIndex + marker.length);

    if (bracketIndex === -1) return [];

    const between = raw.slice(fieldIndex + marker.length, bracketIndex);

    if (!/^\s*:\s*$/.test(between)) return [];

    const items: string[] = [];
    let i = bracketIndex + 1;

    while (i < raw.length) {
        while (i < raw.length && /[\s,]/.test(raw[i])) i++;

        if (raw[i] !== '"') break;

        const start = i + 1;
        let end = -1;

        for (let j = start; j < raw.length; j++) {
            if (raw[j] === '\\') {
                j++;
                continue;
            }

            if (raw[j] === '"') {
                end = j;
                break;
            }
        }

        if (end === -1) break;

        items.push(decodeJsonStringBody(raw.slice(start, end)));
        i = end + 1;
    }

    return items;
};

const toolIcon = (tool: string): React.ReactNode => {
    switch (tool) {
        case 'search_forum':
            return <LuSearch className="size-4" />;
        case 'get_posts':
            return <LuMessageCircle className="size-4" />;
        case 'get_topic_summary':
            return <LuFileText className="size-4" />;
        case 'get_topic_overview':
            return <LuBookOpen className="size-4" />;
        case 'note_candidate':
            return <LuNotebookPen className="size-4" />;
        default:
            return <LuSparkles className="size-4" />;
    }
};

const ActivityRow: React.FC<{
    activity: SummaryActivity;
    isCurrent: boolean;
}> = ({ activity, isCurrent }) => {
    if (activity.kind === 'phase') {
        return (
            <div
                className={`flex items-center gap-2.5 text-sm animate-fadeSlideIn ${
                    isCurrent ? 'text-primary/90' : 'text-primary/50'
                }`}
            >
                <span className="shrink-0">
                    {isCurrent ? (
                        <LuLoader className="size-4 animate-spin" />
                    ) : (
                        <LuSparkles className="size-4 text-primary/40" />
                    )}
                </span>
                <span className={isCurrent ? 'animate-pulse' : ''}>{activity.label}</span>
            </div>
        );
    }

    const { call } = activity;
    const isRunning = call.status === 'running';
    const isError = call.status === 'error';

    return (
        <div
            className={`flex items-start gap-2.5 text-sm animate-fadeSlideIn ${
                isRunning ? 'text-primary/90' : isError ? 'text-red-500/90' : 'text-primary/55'
            }`}
        >
            <span className="shrink-0 mt-0.5">
                {isRunning ? (
                    <LuLoader className="size-4 animate-spin" />
                ) : isError ? (
                    <LuTriangleAlert className="size-4" />
                ) : (
                    <span className="text-primary/40">{toolIcon(call.tool)}</span>
                )}
            </span>
            <span className="min-w-0">
                <span className={isRunning ? 'animate-pulse' : ''}>{call.label}</span>
                {!isRunning && call.detail && (
                    <span className="text-primary/40"> &middot; {call.detail}</span>
                )}
            </span>
            {!isRunning && !isError && (
                <LuCheck className="size-3.5 shrink-0 mt-1 ml-auto text-primary/30" />
            )}
        </div>
    );
};

const ActivityTimeline: React.FC<{
    activities: SummaryActivity[];
    isLive: boolean;
}> = ({ activities, isLive }) => {
    const lastRunningIndex = activities.reduce(
        (last, activity, index) =>
            activity.kind === 'tool' && activity.call.status === 'running' ? index : last,
        -1
    );
    const currentIndex = isLive
        ? lastRunningIndex !== -1
            ? lastRunningIndex
            : activities.length - 1
        : -1;

    return (
        <div className="space-y-2" aria-live="polite">
            {activities.map((activity, index) => (
                <ActivityRow
                    key={activity.activityKey}
                    activity={activity}
                    isCurrent={index === currentIndex}
                />
            ))}
        </div>
    );
};

export type SummaryTabId = 'key_points' | 'open_questions' | 'perspectives';

const PerspectiveGroups: React.FC<{
    groups: PerspectiveGroup[];
    discourseId: string;
}> = ({ groups, discourseId }) => (
    <div className="space-y-4">
        {groups.map((group, groupIndex) => (
            <div
                key={groupIndex}
                className="border-l-2 border-primary/15 pl-3 space-y-2 animate-fadeSlideIn"
            >
                {group.label && (
                    <h4 className="text-xs font-semibold uppercase tracking-wide text-primary/50">
                        {group.label}
                    </h4>
                )}
                {(group.people ?? []).map((person, personIndex) => (
                    <div key={personIndex} className="space-y-0.5">
                        {person.username ? (
                            <Link
                                to="/u/$discourseId/$userId"
                                params={{ discourseId, userId: person.username }}
                                className="text-sm font-medium text-primary hover:underline"
                            >
                                @{person.username}
                            </Link>
                        ) : (
                            <span className="text-sm text-primary/50">A community member</span>
                        )}
                        <p className="text-sm text-primary/70 leading-snug">{person.summary}</p>
                    </div>
                ))}
            </div>
        ))}
    </div>
);

export const SummaryTabs: React.FC<{
    keyPoints: string[];
    openQuestions: string[];
    perspectives: PerspectiveGroup[];
    discourseId: string;
    initialTab?: SummaryTabId;
}> = ({ keyPoints, openQuestions, perspectives, discourseId, initialTab = 'key_points' }) => {
    const [activeTab, setActiveTab] = React.useState<SummaryTabId>(initialTab);
    const perspectiveGroups = perspectives.filter((group) => (group.people?.length ?? 0) > 0);
    const peopleCount = perspectiveGroups.reduce(
        (sum, group) => sum + (group.people?.length ?? 0),
        0
    );

    const allTabs: {
        tabId: SummaryTabId;
        label: string;
        icon: React.ReactNode;
        count: number;
    }[] = [
        {
            tabId: 'key_points',
            label: 'Key Points',
            icon: <LuList className="size-3.5" />,
            count: keyPoints.length,
        },
        {
            tabId: 'open_questions',
            label: 'Open Questions',
            icon: <LuMessageCircle className="size-3.5" />,
            count: openQuestions.length,
        },
        {
            tabId: 'perspectives',
            label: 'Perspectives',
            icon: <LuUsers className="size-3.5" />,
            count: peopleCount,
        },
    ];
    const tabs = allTabs.filter((tab) => tab.count > 0);

    if (tabs.length === 0) return null;

    const current = tabs.find((tab) => tab.tabId === activeTab) ?? tabs[0];

    return (
        <div>
            <div className="flex items-center gap-4 border-b border-primary/10" role="tablist">
                {tabs.map((tab) => (
                    <button
                        key={tab.tabId}
                        role="tab"
                        aria-selected={tab.tabId === current.tabId}
                        className={`flex items-center gap-1.5 pb-1.5 -mb-px text-sm border-b-2 transition-colors ${
                            tab.tabId === current.tabId
                                ? 'border-primary text-primary font-medium'
                                : 'border-transparent text-primary/50 hover:text-primary/80'
                        }`}
                        onClick={() => setActiveTab(tab.tabId)}
                    >
                        {tab.icon}
                        {tab.label}
                        <span className="text-xs text-primary/40">{tab.count}</span>
                    </button>
                ))}
            </div>
            <div className="pt-2.5" role="tabpanel">
                {current.tabId === 'perspectives' ? (
                    <PerspectiveGroups groups={perspectiveGroups} discourseId={discourseId} />
                ) : (
                    <ul className="space-y-1.5">
                        {(current.tabId === 'key_points' ? keyPoints : openQuestions).map(
                            (item, i) => (
                                <li
                                    key={i}
                                    className="text-sm text-primary/70 flex gap-2 animate-fadeSlideIn"
                                >
                                    {current.tabId === 'key_points' ? (
                                        <span className="text-primary/40 mt-0.5 shrink-0">
                                            &bull;
                                        </span>
                                    ) : (
                                        <span className="text-yellow-500 mt-0.5 shrink-0">?</span>
                                    )}
                                    <span>{item}</span>
                                </li>
                            )
                        )}
                    </ul>
                )}
            </div>
        </div>
    );
};

export const StreamingSummary: React.FC<StreamingSummaryProps> = ({
    discourseId,
    topicId,
    mode = 'generate',
    initialTab,
}) => {
    const { combinedContent, activities, error, isComplete, startStream } = useTopicSummaryStream(
        discourseId,
        topicId
    );
    const { mutateAsync: startSummaryGeneration } = useStartTopicSummaryStream();
    const { data: cachedSummary } = useCachedTopicSummary(discourseId, topicId);
    const hasStartedRef = React.useRef(mode === 'cached');
    const forceNextRef = React.useRef(false);
    const [existingSummary, setExistingSummary] = React.useState<string | null>(null);
    const [isUnavailable, setIsUnavailable] = React.useState(false);
    const [phase, setPhase] = React.useState<
        'idle' | 'starting' | 'streaming' | 'complete' | 'error'
    >(mode === 'cached' ? 'complete' : 'idle');
    const [refreshKey, setRefreshKey] = React.useState(0);
    const isAdmin = typeof window !== 'undefined' && !!localStorage.getItem('admin_key');

    const derived = React.useMemo(() => tryParseSummary(combinedContent), [combinedContent]);
    const effectiveExisting =
        existingSummary ??
        (mode === 'cached' && phase === 'complete' ? (cachedSummary?.summary_text ?? null) : null);
    const existingParsed = React.useMemo(
        () => (effectiveExisting ? tryParseSummary(effectiveExisting) : null),
        [effectiveExisting]
    );

    const streamingRaw = React.useMemo(() => {
        if (derived || !combinedContent.trim()) return null;

        return stripCodeFences(combinedContent);
    }, [combinedContent, derived]);
    const partialOverview = streamingRaw ? extractStringField(streamingRaw, 'overview') : null;
    const partialKeyPoints = React.useMemo(
        () => (streamingRaw ? extractCompletedStrings(streamingRaw, 'key_points') : []),
        [streamingRaw]
    );
    const partialOpenQuestions = React.useMemo(
        () => (streamingRaw ? extractCompletedStrings(streamingRaw, 'open_questions') : []),
        [streamingRaw]
    );

    React.useEffect(() => {
        if (hasStartedRef.current) return;

        const startSummary = async () => {
            try {
                hasStartedRef.current = true;
                const force = forceNextRef.current;

                forceNextRef.current = false;
                setPhase('starting');
                const result = await startSummaryGeneration({
                    discourse_id: discourseId,
                    topicId,
                    force,
                });

                if (result.state === 'unavailable') {
                    setIsUnavailable(true);

                    return;
                }

                const { response } = result;

                if (response.status === 'existing' && response.summary) {
                    setExistingSummary(response.summary);
                    setPhase('complete');
                } else {
                    setPhase('streaming');
                    startStream();
                }
            } catch {
                hasStartedRef.current = false;
                setPhase('error');
            }
        };

        startSummary();
    }, [topicId, refreshKey]);

    React.useEffect(() => {
        if (isComplete && !error && phase === 'streaming') setPhase('complete');

        if (error) setPhase('error');
    }, [isComplete, error, phase]);

    // A finished generation supersedes the cached summary everywhere.
    React.useEffect(() => {
        if (!isComplete || error) return;

        queryClient.invalidateQueries({
            queryKey: ['topic-summary-cached', discourseId, topicId],
        });
    }, [isComplete, error, discourseId, topicId]);

    const structured = derived ?? existingParsed;
    // Non-JSON output has no overview field to extract; stream it as-is.
    const isJsonStream = !!streamingRaw && streamingRaw.trimStart().startsWith('{');
    const liveText = derived?.overview ?? (isJsonStream ? partialOverview : streamingRaw);
    const overviewText =
        liveText ?? structured?.overview ?? (existingParsed ? null : effectiveExisting);
    // While regenerating, prefer freshly streamed items over a stale cached set.
    const keyPoints =
        derived?.key_points ??
        (partialKeyPoints.length > 0 ? partialKeyPoints : (structured?.key_points ?? []));
    const openQuestions =
        derived?.open_questions ??
        (partialOpenQuestions.length > 0
            ? partialOpenQuestions
            : (structured?.open_questions ?? []));
    const perspectives = structured?.perspectives ?? [];
    const isLive = phase === 'starting' || phase === 'streaming';

    if (isUnavailable) {
        return <p className="text-sm text-primary/60">Summaries are currently unavailable.</p>;
    }

    return (
        <div className="space-y-4">
            {/* Live activity timeline */}
            {isLive && (
                <div className="space-y-2 py-1">
                    {activities.length > 0 ? (
                        <ActivityTimeline activities={activities} isLive />
                    ) : (
                        <div className="flex items-center gap-2.5 text-sm text-primary/60">
                            <LuLoader className="size-4 animate-spin" />
                            <span className="animate-pulse">Connecting to the summarizer...</span>
                        </div>
                    )}
                </div>
            )}

            {/* Derived data — tabbed, above the narrative */}
            <SummaryTabs
                keyPoints={keyPoints}
                openQuestions={openQuestions}
                perspectives={perspectives}
                discourseId={discourseId}
                initialTab={initialTab}
            />

            {/* Overview — streams in live, then settles */}
            {overviewText && (
                <div className="prose prose-sm max-w-none">
                    <Markdown remarkPlugins={[remarkGfm]}>{overviewText}</Markdown>
                    {phase === 'streaming' && (
                        <span className="animate-pulse text-primary">&#9611;</span>
                    )}
                </div>
            )}

            {phase === 'complete' && structured && (
                <>
                    {structured.changelog_entry?.entry && (
                        <div className="rounded border border-primary/10 bg-primary/5 p-3 space-y-1">
                            <h4 className="text-xs font-medium text-primary/40 uppercase tracking-wide">
                                What changed
                            </h4>
                            <div className="text-sm text-primary/70 prose prose-sm max-w-none">
                                <Markdown remarkPlugins={[remarkGfm]}>
                                    {structured.changelog_entry.entry}
                                </Markdown>
                            </div>
                            {structured.changelog_entry.post_range && (
                                <div className="text-xs text-primary/40">
                                    Posts {structured.changelog_entry.post_range[0]}&ndash;
                                    {structured.changelog_entry.post_range[1]}
                                </div>
                            )}
                        </div>
                    )}
                </>
            )}

            {/* Full trace of how the summary was produced */}
            {phase === 'complete' && activities.length > 0 && (
                <details className="group">
                    <summary className="cursor-pointer text-sm text-primary/50 hover:text-primary/70 transition-colors flex items-center gap-2">
                        <LuBrain className="size-4" />
                        <span>Thinking process</span>
                        <span className="text-primary/30 text-xs">({activities.length} steps)</span>
                    </summary>
                    <div className="mt-3 pl-1">
                        <ActivityTimeline activities={activities} isLive={false} />
                    </div>
                </details>
            )}

            {/* Footer */}
            {phase === 'complete' && (
                <div className="pt-4 border-t border-primary/20 flex items-center justify-between">
                    <p className="text-sm text-primary/40">
                        {existingSummary === null &&
                        mode === 'cached' &&
                        cachedSummary?.based_on_post_number != null
                            ? `Based on the first ${cachedSummary.based_on_post_number} posts`
                            : 'Summary complete'}
                    </p>
                    {isAdmin && (
                        <button
                            className="flex items-center gap-1 text-xs text-primary/40 hover:text-primary transition-colors"
                            onClick={() => {
                                forceNextRef.current = true;
                                hasStartedRef.current = false;
                                setExistingSummary(null);
                                setPhase('idle');
                                setRefreshKey((k) => k + 1);
                            }}
                        >
                            <LuRefreshCw className="size-3" />
                            Regenerate
                        </button>
                    )}
                </div>
            )}

            {error && (
                <div className="text-red-500 text-sm">
                    Error generating summary: {String(error)}
                </div>
            )}
        </div>
    );
};
