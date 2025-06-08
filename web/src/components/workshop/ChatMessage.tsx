import '../../styles/code.css';

import classNames from 'classnames';
import React from 'react';
import { LuBrain, LuChevronLeft, LuChevronRight, LuCog, LuCopy, LuPencil } from 'react-icons/lu';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { match } from 'ts-pattern';

import { extractModelId, useUsageCost } from '@/api/openrouter';
import { components } from '@/api/schema.gen';
import { useWorkshopStreamMessage, WorkshopMessage } from '@/api/workshop';
import { UsageTooltip } from '@/components/usage/UsageTooltip';

import { MarkdownLink } from './components/MarkdownLink';
import { ToolCallDisplay } from './ToolCallDisplay';

// Custom markdown components
const markdownComponents = {
    a: MarkdownLink,
};

// Streaming Events Types (temporary until schema is regenerated)
interface StreamingEvent {
    content: string;
    type: 'content' | 'tool_call_start' | 'tool_call_result' | 'tool_call_error';
    tool_call?: {
        tool_name: string;
        tool_id: string;
        arguments?: string;
        result?: string;
        status: 'starting' | 'executing' | 'success' | 'error';
    };
}

// Component to display stored streaming events using the same logic as live streaming
const StoredStreamingEvents = ({ events }: { events: StreamingEvent[] }) => {
    // Convert stored events to StreamingResponse format to reuse existing logic
    const streamingResponses: components['schemas']['StreamingResponse'][] = events.map(
        (event) => ({
            content: event.content,
            is_complete: false,
            error: undefined,
            entry_type: event.type as any,
            tool_call: event.tool_call
                ? {
                      tool_name: event.tool_call.tool_name,
                      tool_id: event.tool_call.tool_id,
                      arguments: event.tool_call.arguments || undefined,
                      result: event.tool_call.result || undefined,
                      status: event.tool_call.status as any,
                  }
                : undefined,
        })
    );

    // Use the same tool call processing logic as the streaming API
    const toolCalls = React.useMemo(() => {
        const calls = new Map<string, components['schemas']['ToolCallEntry']>();

        streamingResponses.forEach((response) => {
            if (response.tool_call && response.entry_type !== 'Content') {
                calls.set(response.tool_call.tool_id, response.tool_call);
            }
        });

        return Array.from(calls.values());
    }, [streamingResponses]);

    if (toolCalls.length === 0) {
        return null;
    }

    return (
        <div className="mb-6">
            <div className="flex items-center gap-2 mb-4 px-1">
                <div className="flex items-center justify-center w-6 h-6 rounded-md bg-secondary text-primary shadow-sm">
                    <LuCog size={14} />
                </div>
                <h3 className="text-sm font-semibold text-primary">Tool Executions</h3>
                <div className="flex-1 h-px bg-primary/20"></div>
                <span className="text-xs text-primary/70 bg-secondary px-2 py-1 rounded-full border border-primary/20">
                    {toolCalls.length} call{toolCalls.length !== 1 ? 's' : ''}
                </span>
            </div>
            <div className="space-y-3">
                {toolCalls.map((toolCall) => (
                    <ToolCallDisplay key={toolCall.tool_id} toolCall={toolCall} />
                ))}
            </div>
        </div>
    );
};

// Helper function to safely convert unknown streaming_events to typed version
const convertToExtendedMessage = (message: WorkshopMessage): ExtendedWorkshopMessage => {
    const { streaming_events, ...rest } = message;
    let typedStreamingEvents: StreamingEvent[] | undefined;

    // Type guard for streaming events
    if (streaming_events && Array.isArray(streaming_events)) {
        typedStreamingEvents = streaming_events.filter(
            (event: any) =>
                event && typeof event.content === 'string' && typeof event.type === 'string'
        ) as StreamingEvent[];
    }

    return {
        ...rest,
        streaming_events: typedStreamingEvents,
    };
};

// Extended WorkshopMessage type that includes streaming_events
interface ExtendedWorkshopMessage extends Omit<WorkshopMessage, 'streaming_events'> {
    streaming_events?: StreamingEvent[];
}

export interface MessageTreeNode {
    message: ExtendedWorkshopMessage;
    children: MessageTreeNode[];
    siblings: ExtendedWorkshopMessage[];
    currentSiblingIndex: number;
}

export interface ChatMessageProps {
    node?: MessageTreeNode;
    message?: ExtendedWorkshopMessage;
    onEdit?: (message: ExtendedWorkshopMessage) => void;
    onNavigate?: (message: ExtendedWorkshopMessage) => void;
}

export const ChatMessage = ({ node, message, onEdit, onNavigate }: ChatMessageProps) => {
    // Support both old and new interfaces
    const messageData = node?.message || convertToExtendedMessage(message!);
    const siblings = node?.siblings || [];
    const currentSiblingIndex = node?.currentSiblingIndex || 0;
    const hasPrevSibling = currentSiblingIndex > 0;
    const hasNextSibling = currentSiblingIndex < siblings.length - 1;

    const handlePrevSibling = () => {
        if (hasPrevSibling && onNavigate) {
            onNavigate(siblings[currentSiblingIndex - 1]);
        }
    };

    const handleNextSibling = () => {
        if (hasNextSibling && onNavigate) {
            onNavigate(siblings[currentSiblingIndex + 1]);
        }
    };

    const handleEdit = () => {
        if (onEdit) {
            onEdit(messageData);
        }
    };

    const handleCopy = () => {
        navigator.clipboard.writeText(messageData.message);
    };

    return (
        <div
            className={classNames(
                'flex flex-col gap-2 scroll-m-20',
                messageData.sender_role === 'user' && 'ml-auto w-fit',
                messageData.sender_role === 'assistant' && ''
            )}
            key={messageData.message_id}
            id={messageData.message_id}
        >
            {match(messageData.sender_role)
                .with('user', () => <div className="text-sm text-primary/50">You</div>)
                .with('assistant', () => <div className="text-sm text-primary/50">Assistant</div>)
                .otherwise(() => null)}

            <div
                key={messageData.message_id}
                // className="border p-4 border-primary/50 rounded-md pr-6"
            >
                {/* Show stored streaming events if they exist */}
                {messageData.streaming_events && messageData.streaming_events.length > 0 && (
                    <StoredStreamingEvents events={messageData.streaming_events} />
                )}
                <div
                    className={classNames(
                        'prose',
                        messageData.sender_role === 'user' && 'card border border-primary/50'
                    )}
                >
                    <Markdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                        {messageData.message}
                    </Markdown>
                </div>
                {messageData.message.length === 0 && (
                    <ChatDataStream
                        chatId={messageData.chat_id}
                        messageId={messageData.message_id}
                    />
                )}
            </div>

            {/* Branch navigation and actions */}
            <div className="text-sm text-primary/50 flex justify-between gap-2 items-center">
                {/* Token usage display */}
                <div className="flex-1">
                    <TokenUsageDisplay message={messageData} />
                </div>

                <div className="flex gap-2 items-center">
                    {/* Branch navigation arrows - only show if using tree interface */}
                    {node && siblings.length > 1 && (
                        <>
                            <button
                                className={classNames(
                                    'button aspect-square size-8 flex justify-center items-center',
                                    !hasPrevSibling && 'opacity-30 cursor-not-allowed'
                                )}
                                onClick={handlePrevSibling}
                                disabled={!hasPrevSibling}
                                title="Previous branch"
                            >
                                <LuChevronLeft />
                            </button>
                            <span className="text-xs px-1">
                                {currentSiblingIndex + 1}/{siblings.length}
                            </span>
                            <button
                                className={classNames(
                                    'button aspect-square size-8 flex justify-center items-center',
                                    !hasNextSibling && 'opacity-30 cursor-not-allowed'
                                )}
                                onClick={handleNextSibling}
                                disabled={!hasNextSibling}
                                title="Next branch"
                            >
                                <LuChevronRight />
                            </button>
                        </>
                    )}

                    {/* Standard actions */}
                    <button
                        className="button gap-2 aspect-square size-8 flex justify-center items-center"
                        onClick={handleCopy}
                        title="Copy message"
                    >
                        <LuCopy />
                    </button>
                    {match(messageData.sender_role)
                        .with('user', () => (
                            <button
                                className="button gap-2 aspect-square size-8 flex justify-center items-center"
                                onClick={handleEdit}
                                title="Edit message"
                                disabled={!onEdit}
                            >
                                <LuPencil />
                            </button>
                        ))
                        .otherwise(() => null)}
                </div>
            </div>
        </div>
    );
};

export const ChatDataStream = ({ chatId, messageId }: { chatId: string; messageId: string }) => {
    const { combinedContent, toolCalls, isLoading, error, isComplete } = useWorkshopStreamMessage(
        chatId,
        messageId
    );

    if (isLoading)
        return (
            <div className="flex items-center gap-2">
                <LuBrain />
                Thinking...
            </div>
        );

    if (error) return <div>Error: {error}</div>;

    return (
        <>
            {/* Display tool calls */}
            {toolCalls.length > 0 && (
                <div className="mb-6">
                    <div className="flex items-center gap-2 mb-4 px-1">
                        <div className="flex items-center justify-center w-6 h-6 rounded-md bg-secondary text-primary shadow-sm">
                            <LuCog size={14} />
                        </div>
                        <h3 className="text-sm font-semibold text-primary">Tool Executions</h3>
                        <div className="flex-1 h-px bg-primary/20"></div>
                        <span className="text-xs text-primary/70 bg-secondary px-2 py-1 rounded-full border border-primary/20">
                            {toolCalls.length} call{toolCalls.length !== 1 ? 's' : ''}
                        </span>
                    </div>
                    <div className="space-y-3">
                        {toolCalls.map((toolCall) => (
                            <ToolCallDisplay key={toolCall.tool_id} toolCall={toolCall} />
                        ))}
                    </div>
                </div>
            )}

            {/* Display regular content */}
            <div className="prose">
                <Markdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                    {combinedContent}
                </Markdown>
            </div>
            {!isComplete && !error && <span className="animate-pulse">▋</span>}
        </>
    );
};

// Token Usage Display Component
const TokenUsageDisplay = ({ message }: { message: WorkshopMessage }) => {
    // Only show for assistant messages that have token data
    if (message.sender_role !== 'assistant' || !message.total_tokens) {
        return null;
    }

    // Extract model ID and calculate costs
    const modelId = extractModelId(message.model_used);
    const usageCost = useUsageCost(
        {
            prompt_tokens: message.prompt_tokens,
            completion_tokens: message.completion_tokens,
            reasoning_tokens: message.reasoning_tokens,
            total_tokens: message.total_tokens,
        },
        modelId
    );

    const inputTokens = message.prompt_tokens || 0;
    const outputTokens = message.completion_tokens || 0;
    const reasoningTokens = message.reasoning_tokens || 0;

    return (
        <UsageTooltip
            inputTokens={inputTokens}
            outputTokens={outputTokens}
            reasoningTokens={reasoningTokens}
            totalTokens={message.total_tokens}
            usageCost={usageCost}
            modelUsed={message.model_used}
        />
    );
};
