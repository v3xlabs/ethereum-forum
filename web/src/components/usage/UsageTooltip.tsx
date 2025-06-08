import React, { FC, useEffect, useState } from 'react';
import * as RadixPopover from '@radix-ui/react-popover';
import * as RadixTooltip from '@radix-ui/react-tooltip';
import { FiActivity } from 'react-icons/fi';

import { UsageCost } from '@/api/openrouter';

const formatNumber = (num: number) => num.toLocaleString();

// Token Usage Bar Component
const TokenUsageBar = ({
    inputTokens,
    outputTokens,
    reasoningTokens,
}: {
    inputTokens: number;
    outputTokens: number;
    reasoningTokens?: number;
}) => {
    const totalTokens = inputTokens + outputTokens + (reasoningTokens || 0);
    const inputPercentage = (inputTokens / totalTokens) * 100;
    const outputPercentage = (outputTokens / totalTokens) * 100;
    const reasoningPercentage = reasoningTokens ? (reasoningTokens / totalTokens) * 100 : 0;

    return (
        <div className="w-full h-3 bg-primary/10 rounded-full overflow-hidden">
            {/* Input tokens - blue */}
            <div className="flex h-full">
                <div
                    className="h-full bg-blue-400 transition-all duration-300"
                    style={{ width: `${inputPercentage}%` }}
                />
                {/* Reasoning tokens - purple */}
                {(reasoningTokens && reasoningTokens > 0 && (
                    <div
                        className="h-full bg-purple-400 transition-all duration-300"
                        style={{ width: `${reasoningPercentage}%` }}
                    />
                )) ||
                    undefined}
                {/* Output tokens - green */}
                <div
                    className="h-full bg-green-400 transition-all duration-300"
                    style={{ width: `${outputPercentage}%` }}
                />
            </div>
        </div>
    );
};

export interface UsageTooltipProps {
    inputTokens: number;
    outputTokens: number;
    reasoningTokens?: number;
    totalTokens: number;
    usageCost?: UsageCost | null;
    modelUsed?: string;
}

export const UsageTooltipContent: FC<UsageTooltipProps> = ({
    inputTokens,
    outputTokens,
    reasoningTokens = 0,
    totalTokens,
    usageCost,
    modelUsed,
}) => {
    return (
        <div className="px-4 py-1 space-y-3 max-w-sm">
            {/* Header with activity icon */}
            <div className="flex items-center gap-2 font-semibold text-sm">
                <FiActivity className="w-4 h-4 text-primary/70" />
                <span>Token Usage & Cost</span>
            </div>

            {/* Visual breakdown with bar */}
            <TokenUsageBar
                inputTokens={inputTokens}
                outputTokens={outputTokens}
                reasoningTokens={reasoningTokens}
            />

            {/* Concise breakdown */}
            <div className="space-y-2 text-xs">
                {/* Token breakdown */}
                {inputTokens > 0 && (
                    <div className="flex justify-between items-center">
                        <span className="flex items-center gap-2">
                            <div className="w-2 h-2 bg-blue-400 rounded"></div>
                            <span>Input</span>
                        </span>
                        <div className="flex gap-3 items-center">
                            <span className="font-mono">{formatNumber(inputTokens)}</span>
                            {usageCost && (
                                <span className="text-green-600 font-mono text-xs">
                                    {usageCost.formattedPromptCost}
                                </span>
                            )}
                        </div>
                    </div>
                )}

                {outputTokens > 0 && (
                    <div className="flex justify-between items-center">
                        <span className="flex items-center gap-2">
                            <div className="w-2 h-2 bg-green-400 rounded"></div>
                            <span>Output</span>
                        </span>
                        <div className="flex gap-3 items-center">
                            <span className="font-mono">{formatNumber(outputTokens)}</span>
                            {usageCost && (
                                <span className="text-green-600 font-mono text-xs">
                                    {usageCost.formattedCompletionCost}
                                </span>
                            )}
                        </div>
                    </div>
                )}

                {reasoningTokens > 0 && (
                    <div className="flex justify-between items-center">
                        <span className="flex items-center gap-2">
                            <div className="w-2 h-2 bg-purple-400 rounded"></div>
                            <span>Reasoning</span>
                        </span>
                        <div className="flex gap-3 items-center">
                            <span className="font-mono">{formatNumber(reasoningTokens)}</span>
                            {usageCost && (
                                <span className="text-green-600 font-mono text-xs">
                                    {usageCost.formattedReasoningCost}
                                </span>
                            )}
                        </div>
                    </div>
                )}

                {/* Total */}
                <div className="border-t border-secondary pt-2 flex justify-between items-center font-semibold">
                    <span>Total</span>
                    <div className="flex gap-3 items-center">
                        <span className="font-mono">{formatNumber(totalTokens)}</span>
                        {usageCost && (
                            <span className="text-green-600 font-mono">
                                {usageCost.formattedTotalCost}
                            </span>
                        )}
                    </div>
                </div>
            </div>

            {/* Model information */}
            {modelUsed && (
                <div className="border-t border-secondary pt-2">
                    <div className="flex justify-between gap-4 text-xs">
                        <span className="text-primary/70">Model:</span>
                        <a
                            href={`https://openrouter.ai/models/${modelUsed}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="font-mono text-xs break-all text-right link"
                        >
                            {modelUsed}
                        </a>
                    </div>
                </div>
            )}
        </div>
    );
};

export const UsageTooltip: FC<UsageTooltipProps> = (props) => {
    const [isTouch, setIsTouch] = useState(false);

    useEffect(() => {
        if (typeof window !== 'undefined') {
            setIsTouch(window.matchMedia('(pointer: coarse)').matches);
        }
    }, []);

    const trigger = (
        <div className="inline-flex items-center gap-2 text-xs text-primary/60 bg-secondary/30 px-2 py-1 rounded-full border border-secondary/50 hover:bg-secondary/50 transition-colors">
            <span className="font-medium">{formatNumber(props.totalTokens)} tokens</span>
            {props.usageCost && (
                <>
                    <div className="w-px h-3 bg-primary/20" />
                    <span className="text-green-700 font-medium text-xs">
                        {props.usageCost.totalCost === 0n ? 'Free' : props.usageCost.formattedTotalCost}
                    </span>
                </>
            )}
        </div>
    );

    const content = <UsageTooltipContent {...props} />;

    if (isTouch) {
        return (
            <RadixPopover.Root>
                <RadixPopover.Trigger asChild>{trigger}</RadixPopover.Trigger>
                <RadixPopover.Portal>
                    <RadixPopover.Content
                        sideOffset={5}
                        className="bg-secondary border border-secondary text-primary p-2 rounded-md max-w-sm z-50 text-sm text"
                    >
                        {content}
                        <RadixPopover.Arrow className="fill-secondary" />
                    </RadixPopover.Content>
                </RadixPopover.Portal>
            </RadixPopover.Root>
        );
    }

    return (
        <RadixTooltip.Provider delayDuration={300}>
            <RadixTooltip.Root>
                <RadixTooltip.Trigger asChild>{trigger}</RadixTooltip.Trigger>
                <RadixTooltip.Portal>
                    <RadixTooltip.Content
                        className="bg-secondary border border-secondary text-primary p-2 rounded-md max-w-sm z-50 text-sm text"
                        sideOffset={5}
                    >
                        {content}
                        <RadixTooltip.Arrow className="fill-secondary" />
                    </RadixTooltip.Content>
                </RadixTooltip.Portal>
            </RadixTooltip.Root>
        </RadixTooltip.Provider>
    );
};
