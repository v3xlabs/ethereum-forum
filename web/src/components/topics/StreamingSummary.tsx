import { useNavigate } from '@tanstack/react-router';
import React from 'react';
import { LuBrain, LuLoader } from 'react-icons/lu';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import { useAuth } from '@/api/auth';
import {
    useDeleteTopicSummary,
    useStartTopicSummaryStream,
    useTopicSummaryStream,
} from '@/api/topics';
import { useWorkshopCreateChatFromSummary } from '@/api/workshop';

interface StreamingSummaryProps {
    discourseId: string;
    topicId: number;
}

export const StreamingSummary: React.FC<StreamingSummaryProps> = ({ discourseId, topicId }) => {
    const { isAuthenticated } = useAuth();
    const { combinedContent, isLoading, error, isComplete, startStream } = useTopicSummaryStream(
        discourseId,
        topicId
    );

    const navigate = useNavigate();
    const { mutateAsync: createChatFromSummary } = useWorkshopCreateChatFromSummary();
    const { mutateAsync: deleteTopicSummary } = useDeleteTopicSummary();

    const { mutateAsync: startSummaryGeneration } = useStartTopicSummaryStream();
    const hasStartedRef = React.useRef(false);
    const [existingSummary, setExistingSummary] = React.useState<string | null>(null);
    const [showExisting, setShowExisting] = React.useState(false);

    React.useEffect(() => {
        // Only start once per topic
        if (hasStartedRef.current) return;

        const startSummary = async () => {
            try {
                hasStartedRef.current = true;
                // First trigger the summary generation on the backend
                const response = await startSummaryGeneration({
                    discourse_id: discourseId,
                    topicId,
                });

                if (response.status === 'existing' && response.summary) {
                    // We have an existing summary, show it immediately
                    setExistingSummary(response.summary);
                    setShowExisting(true);
                } else if (response.status === 'ongoing' || response.status === 'started') {
                    // Start streaming
                    startStream();
                }
            } catch (error) {
                console.error('Error starting summary:', error);
                hasStartedRef.current = false; // Reset on error so user can retry
            }
        };

        startSummary();
    }, [topicId]); // Only depend on topicId, not the function references

    // Show existing summary if we have one and no streaming content
    const displayContent = showExisting && !combinedContent ? existingSummary : combinedContent;
    const isShowingExisting = showExisting && !combinedContent;

    return (
        <div className="space-y-4">
            {isLoading && !isShowingExisting && (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <LuLoader className="w-4 h-4 animate-spin" />
                    Generating summary...
                </div>
            )}

            <div className="prose prose-sm max-w-none">
                {error ? (
                    <div className="text-red-500">Error: {error}</div>
                ) : (
                    <>
                        <Markdown remarkPlugins={[remarkGfm]}>{displayContent || ''}</Markdown>
                        {!displayContent && !isLoading && (
                            <div className="flex items-center gap-2">
                                <LuBrain />
                                <span>Thinking...</span>
                            </div>
                        )}
                        {!isComplete && !error && combinedContent && (
                            <span className="animate-pulse text-primary">▋</span>
                        )}
                    </>
                )}
            </div>

            {(isComplete || isShowingExisting) && displayContent && (
                <div className="pt-4 border-t border-primary/20 flex justify-between items-center">
                    <p className="text-sm text-muted-foreground">
                        {isShowingExisting ? 'Cached summary' : 'Summary complete'}
                    </p>
                    <div className="flex gap-2 text-sm">
                        {isAuthenticated && (
                            <button
                                className="button"
                                onClick={() =>
                                    createChatFromSummary(
                                        { topicId },
                                        {
                                            onSuccess: (data) =>
                                                navigate({
                                                    to: '/chat/$chatId',
                                                    params: { chatId: data.chat_id },
                                                    hash: data.message_id,
                                                }),
                                        }
                                    )
                                }
                            >
                                Open in chat
                            </button>
                        )}
                        {localStorage.getItem('forum.ethereum.adminKey') && (
                            <button
                                className="button"
                                onClick={async () => {
                                    try {
                                        const key = localStorage.getItem('forum.ethereum.adminKey');

                                        if (!key) {
                                            console.warn(
                                                'No admin key found, cannot regenerate summary'
                                            );

                                            return;
                                        }

                                        await deleteTopicSummary({
                                            discourse_id: discourseId,
                                            topicId,
                                        });

                                        setExistingSummary(null);
                                        setShowExisting(false);
                                        hasStartedRef.current = false;

                                        const response = await startSummaryGeneration({
                                            discourse_id: discourseId,
                                            topicId,
                                        });

                                        if (
                                            response.status === 'ongoing' ||
                                            response.status === 'started'
                                        ) {
                                            startStream();
                                        }
                                    } catch (error) {
                                        console.error('Error regenerating summary:', error);
                                    }
                                }}
                            >
                                Regenerate
                            </button>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
};
