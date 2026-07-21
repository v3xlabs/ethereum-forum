import { createFileRoute } from '@tanstack/react-router';
import { LuBook, LuGithub, LuWandSparkles } from 'react-icons/lu';

import { ActivityDigest } from '@/components/topic/ActivityDigest';
import { TopicList } from '@/components/topic/TopicList';
import { TopicsTrending } from '@/components/topic/TopicsTrending';

export const Route = createFileRoute('/')({
    component: () => <RouteComponent />,
});

const RouteComponent = () => {
    return (
        <>
            <div className="mx-auto w-full max-w-6xl space-y-6 px-3 pt-8">
                <main className="space-y-6">
                    <ActivityDigest />
                    <TopicsTrending />
                    <TopicList />
                </main>
                <footer className="flex w-full items-center justify-center gap-4 pb-8 text-sm">
                    <div className="flex items-center gap-1">
                        <a
                            href="/docs"
                            className="hover:text-secondary transition-colors flex items-center gap-1"
                            target="_blank"
                            rel="noreferrer"
                        >
                            <LuBook className="size-4" />
                            <span>Docs</span>
                        </a>
                        <div>
                            (
                            <a href="/openapi.json" className="link">
                                <span>openapi.json</span>
                            </a>
                            )
                        </div>
                    </div>
                    <a
                        href="https://ethereum-magicians.org/"
                        className="hover:text-secondary transition-colors flex items-center gap-1"
                        target="_blank"
                        rel="noreferrer"
                    >
                        <LuWandSparkles className="size-4" />
                        <span>Ethereum Magicians</span>
                    </a>
                    <a
                        href="https://github.com/v3xlabs/ethereum-forum"
                        className="hover:text-secondary transition-colors flex items-center gap-1"
                        target="_blank"
                        rel="noreferrer"
                    >
                        <LuGithub className="size-4" />
                        <span>Contribute</span>
                    </a>
                </footer>
            </div>
        </>
    );
};
