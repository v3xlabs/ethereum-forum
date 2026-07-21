import { createFileRoute } from '@tanstack/react-router';

import { useSystemPrompts } from '@/api/admin';

const PromptSection = ({ title, content }: { title: string; content: string }) => (
    <div className="space-y-2">
        <h2 className="text-lg font-medium">{title}</h2>
        <pre className="p-4 rounded bg-primary/5 border border-primary/20 text-sm overflow-x-auto whitespace-pre-wrap max-h-96 overflow-y-auto">
            {content}
        </pre>
    </div>
);

const AdminPromptsPage = () => {
    const { data: prompts, isLoading, error } = useSystemPrompts();

    return (
        <div className="space-y-6">
            <h1 className="text-2xl font-semibold">System Prompts</h1>
            <p className="text-sm text-primary/60">
                These prompts are injected into every LLM run. To update, modify the markdown files
                in the backend.
            </p>
            {isLoading && (
                <div className="space-y-4">
                    {[0, 1, 2].map((index) => (
                        <div key={index} className="h-32 rounded bg-primary/5 animate-pulse" />
                    ))}
                </div>
            )}
            {error && <div className="text-red-500">Error: {String(error)}</div>}
            {prompts && (
                <div className="space-y-8">
                    <PromptSection title="Summary Prompt" content={prompts.summary_prompt} />
                    <PromptSection title="Digest Prompt" content={prompts.digest_prompt} />
                    <PromptSection title="Curator Prompt" content={prompts.curator_prompt} />
                </div>
            )}
        </div>
    );
};

export const Route = createFileRoute('/admin/prompts')({
    component: AdminPromptsPage,
});
