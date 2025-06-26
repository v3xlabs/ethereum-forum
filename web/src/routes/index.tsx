import { createFileRoute } from '@tanstack/react-router';
import { LuBook, LuGithub, LuWandSparkles } from 'react-icons/lu';

import { TopicList } from '@/components/topic/TopicList';
import { TopicsTrending } from '@/components/topic/TopicsTrending';

export const Route = createFileRoute('/')({
    component: RouteComponent,
});

function RouteComponent() {
    return (
        <>
            {/* <div className="right-bar p-4"> */}
            {/* <ProtocolAgendaUpcoming /> */}
            {/* </div> */}
            <div className="mx-auto w-full max-w-screen-lg pt-8 px-2 space-y-4">
                <div className="space-y-4 mx-auto">
                    {/* <div className="card flex-1 flex flex-col gap-1 h-fit col-span-full w-full">
                        <h1 className="">Welcome to Ethereum Forum!</h1>
                        <p className="text-secondary">
                            Lorem ipsum dolor sit amet consectetur adipisicing elit. Quisquam, quos.
                        </p>
                    </div> */}
                    <TopicsTrending />
                    <TopicList />
                </div>
                <div className="w-full flex items-center justify-center gap-4 text-sm pb-8">
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
                </div>
            </div>
        </>
    );
}
