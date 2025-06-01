import { DialogClose } from '@radix-ui/react-dialog';
import { LuRefreshCcw } from 'react-icons/lu';
import Markdown from 'react-markdown';

import { useTopicSummary } from '@/api/topics';

export const TopicSummary = ({ topicId }: { topicId: number }) => {
    const { data: summary, isPending } = useTopicSummary(topicId);

    if (isPending) {
        return (
            <div className="flex items-center gap-2 py-3 px-1.5">
                <div className="animate-spin">
                    <LuRefreshCcw className="size-4" />
                </div>
                <span className="text-sm">Generating summary...</span>
            </div>
        );
    }

    if (!summary) {
        return (
            <div className="text-primary text-sm py-2 px-1.5 italic">
                No summary available for this topic
            </div>
        );
    }

    return (
        <>
            <div className="text-sm leading-relaxed text-primary prose">
                <div className="max-h-[80vh] overflow-scroll">
                    <Markdown>{summary.summary_text.replace(/\\n/g, '\n')}</Markdown>
                </div>
            </div>
            <DialogClose>
                <button className="button">Close</button>
            </DialogClose>
            <button className="button">Open in chat</button>
        </>
    );
};
