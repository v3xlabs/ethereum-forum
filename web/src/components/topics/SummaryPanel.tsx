import * as Dialog from '@radix-ui/react-dialog';
import React from 'react';
import { LuX } from 'react-icons/lu';

import { StreamingSummary, SummaryTabId } from './StreamingSummary';

export const SummaryPanel: React.FC<{
    discourseId: string;
    topicId: number;
    open: boolean;
    onOpenChange: (open: boolean) => void;
    mode: 'generate' | 'cached';
    initialTab?: SummaryTabId;
}> = ({ discourseId, topicId, open, onOpenChange, mode, initialTab }) => (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
        <Dialog.Portal>
            <Dialog.Overlay className="fixed inset-0 z-40" />
            <Dialog.Content className="fixed right-0 top-0 bottom-0 z-50 w-full max-w-md bg-primary border-l border-secondary shadow-[var(--shadow-6)] overflow-y-auto p-5 space-y-4 focus:outline-none data-[state=open]:animate-slideInRight">
                <div className="flex items-center justify-between gap-2">
                    <Dialog.Title className="text-lg font-bold">Topic Summary</Dialog.Title>
                    <Dialog.Close className="hover:bg-secondary rounded-full p-1 shrink-0">
                        <LuX className="size-5" />
                    </Dialog.Close>
                </div>
                <StreamingSummary
                    discourseId={discourseId}
                    topicId={topicId}
                    mode={mode}
                    initialTab={initialTab}
                />
            </Dialog.Content>
        </Dialog.Portal>
    </Dialog.Root>
);
