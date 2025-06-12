import * as Dialog from '@radix-ui/react-dialog';
import { createFileRoute } from '@tanstack/react-router';
import classNames from 'classnames';
import { parseISO } from 'date-fns';
import { Fragment, useEffect } from 'react';
import { FiEye, FiHeart, FiMessageSquare } from 'react-icons/fi';
import {
    LuGithub,
    LuLink,
    LuMessageCircle,
    LuNotebook,
    LuPaperclip,
    LuRefreshCcw,
    LuSignal,
    LuSignalHigh,
    LuSignalLow,
    LuSignalMedium,
    LuSignalZero,
    LuWandSparkles,
    LuX,
    LuYoutube,
} from 'react-icons/lu';
import { PiReceipt } from 'react-icons/pi';
import { SiEthereum, SiOpenai, SiReddit } from 'react-icons/si';
import { toast } from 'sonner';

import { getTopic, usePostsInfinite, useTopic, useTopicRefresh } from '@/api/topics';
import { CategoryTag } from '@/components/CategoryTag';
import { DiscourseInstanceName } from '@/components/DiscourseInstanceIcon';
import { ExpandableList } from '@/components/list/ExpandableList';
import { TimeAgo } from '@/components/TimeAgo';
import { ImageLightbox } from '@/components/topic/Prose';
import { TopicPost } from '@/components/topic/TopicPost';
import { StreamingSummary } from '@/components/topics/StreamingSummary';
import { UpDownScroller } from '@/components/UpDown';
import { decodeCategory } from '@/util/category';
import { isGithub, isHackmd, isStandardsLink, spliceRelatedLinks } from '@/util/links';
import { formatBigNumber } from '@/util/numbers';
import { queryClient } from '@/util/query';

interface DiscourseUser {
    id: number;
    // stylistic
    name: string;
    // real
    username: string;
    avatar_template: string;
}

export const Route = createFileRoute('/t/$discourseId/$topicId/')({
    component: RouteComponent,
    beforeLoad: async ({ params }) => {
        const topic = await queryClient.ensureQueryData(
            getTopic(params.discourseId, params.topicId)
        );

        return {
            title: topic?.title,
        };
    },
});

type RelevantLink = {
    url: string;
    title: string;
    internal: boolean;
    attachment: boolean;
    reflection: boolean;
    clicks: number;
    domain: string;
    root_domain: string;
};

function RouteComponent() {
    const { discourseId, topicId } = Route.useParams();
    const { data: topic } = useTopic(discourseId, topicId);

    const { data, fetchNextPage, hasNextPage, isFetchingNextPage, status } = usePostsInfinite(
        discourseId,
        topicId
    );

    const extra = topic?.extra as Record<string, unknown>;
    const tags = decodeCategory(extra?.['category_id'] as number);

    const all_links = ((extra?.details?.links || []) as RelevantLink[]).sort(
        (a, b) => b.clicks - a.clicks
    );
    const [standards_links, relevant_links1] = spliceRelatedLinks(all_links, (link) =>
        isStandardsLink(link.url)
    );
    const [github_links, relevant_links] = spliceRelatedLinks(relevant_links1, (link) =>
        isGithub(link.url)
    );
    const creator = extra?.details?.created_by as DiscourseUser;

    useEffect(() => {
        document.documentElement.classList.add('prose-page');

        return () => {
            document.documentElement.classList.remove('prose-page');
        };
    }, []);

    return (
        <>
            <ImageLightbox />
            <div className="right-bar p-4 space-y-4">
                <div className="space-y-1.5">
                    <div className="px-1.5">
                        <h3 className="font-bold w-full border-b border-b-primary pb-1">
                            Thread Info
                        </h3>
                    </div>
                    <ul>
                        {creator && (
                            <li className="flex items-center gap-1 mx-1.5 justify-between">
                                <div className="text-base">Author</div>
                                <a
                                    href={'https://ethereum-magicians.org/u/' + creator.username}
                                    className="flex items-center gap-1 hover:bg-secondary w-fit justify-end"
                                    target="_blank"
                                    rel="noreferrer"
                                >
                                    <div className="size-4 rounded-full overflow-hidden">
                                        <img
                                            src={
                                                'https://ethereum-magicians.org' +
                                                creator.avatar_template.replace('{size}', '48')
                                            }
                                            alt={creator.name}
                                            className="w-full h-full object-cover"
                                        />
                                    </div>
                                    <div className="text-base truncate">{creator.name}</div>
                                </a>
                            </li>
                        )}
                        <li className="flex items-center gap-1 px-1.5 justify-between">
                            <div className="text-base">Created</div>
                            <div className="text-base">
                                {topic?.created_at && <TimeAgo date={parseISO(topic.created_at)} />}
                            </div>
                        </li>
                        {topic?.last_post_at && (
                            <li className="flex items-center gap-1 px-1.5 justify-between">
                                <div className="text-base">Last Post</div>
                                <div className="text-base flex items-center gap-1">
                                    <TimeAgo date={parseISO(topic.last_post_at)} />
                                    <SignalIndicator date={parseISO(topic.last_post_at)} />
                                </div>
                            </li>
                        )}
                        <li className="flex items-center gap-1 px-1.5 justify-between">
                            <div className="text-base">Source</div>
                            <div className="text-base flex items-center">
                                <a
                                    href={'https://ethereum-magicians.org/t/' + topic?.topic_id}
                                    target="_blank"
                                    rel="noreferrer"
                                    className="hover:underline"
                                >
                                    <DiscourseInstanceName discourse_id={discourseId} />/
                                    {topic?.topic_id}
                                </a>
                                <RefreshTopicButton discourseId={discourseId} topicId={topicId} />
                            </div>
                        </li>
                        {topic?.topic_id && (
                            <li className="space-y-1.5">
                                <Dialog.Root>
                                    <Dialog.Trigger asChild>
                                        <button
                                            className={classNames(
                                                'w-full text-left flex items-center gap-2 px-2 py-1 rounded-md',
                                                'bg-purple-100 border-purple-300 border text-purple-800 transition-colors duration-200 hover:bg-purple-200',
                                                // html.dark
                                                '[&:is(html.dark_&)]:bg-purple-900/30',
                                                '[&:is(html.dark_&)]:border-purple-700',
                                                '[&:is(html.dark_&)]:text-purple-200',
                                                '[&:is(html.dark_&)]:hover:bg-purple-900/50',
                                                // html.system when dark mode is enabled
                                                '[&:is(html.system_&)]:bg-purple-900/30',
                                                '[&:is(html.system_&)]:border-purple-700',
                                                '[&:is(html.system_&)]:text-purple-200',
                                                '[&:is(html.system_&)]:hover:bg-purple-900/50'
                                            )}
                                        >
                                            <LuWandSparkles />
                                            View Summary
                                        </button>
                                    </Dialog.Trigger>
                                    <Dialog.Portal>
                                        <Dialog.Overlay className="fixed inset-0 bg-black/50 z-40 data-[state=open]:animate-overlayShow overflow-y-scroll grid place-items-center">
                                            <Dialog.Content className="z-50 relative my-10 max-w-3xl shadow-[var(--shadow-6)] focus:outline-none data-[state=open]:animate-contentShow mx-auto w-full p-6 bg-primary space-y-4">
                                                <Dialog.Title className="text-xl font-bold">
                                                    Topic Summary
                                                </Dialog.Title>
                                                <StreamingSummary
                                                    discourseId={discourseId}
                                                    topicId={topic.topic_id}
                                                />
                                                <Dialog.Close className="absolute top-2 right-2 -translate-y-1/2 hover:bg-secondary rounded-full p-1">
                                                    <LuX className="size-5" />
                                                </Dialog.Close>
                                            </Dialog.Content>
                                        </Dialog.Overlay>
                                    </Dialog.Portal>
                                </Dialog.Root>
                            </li>
                        )}
                    </ul>
                </div>
                {/* Links */}
                {standards_links.length > 0 && (
                    <ExpandableList title="Standards Links" maxItems={4}>
                        {standards_links.map((link) => (
                            <li key={link.url}>
                                <RelevantLink link={link} />
                            </li>
                        ))}
                    </ExpandableList>
                )}
                {github_links.length > 0 && (
                    <ExpandableList title="Github Links" maxItems={4}>
                        {github_links.map((link) => (
                            <li key={link.url}>
                                <RelevantLink link={link} />
                            </li>
                        ))}
                    </ExpandableList>
                )}
                {relevant_links.length > 0 && (
                    <ExpandableList title="Related Links" maxItems={4}>
                        {relevant_links?.map((link) => (
                            <li key={link.url}>
                                <RelevantLink link={link} />
                            </li>
                        ))}
                    </ExpandableList>
                )}
            </div>
            <div className="mx-auto w-full prose-width pt-8 px-2 space-y-4 relative">
                <UpDownScroller />
                <div>
                    <div className="flex items-top justify-between gap-2">
                        <div>
                            <h1 className="text-2xl">
                                <b>{topic?.title}</b>
                            </h1>
                            <div className="flex items-center gap-2">
                                {tags?.map((tag) => <CategoryTag key={tag} tag={tag} />)}
                            </div>
                        </div>
                        <div className="flex items-center gap-2">
                            <a
                                href={`https://ethereum-magicians.org/t/${topic?.topic_id}`}
                                className="button aspect-square flex gap-1 items-center"
                                target="_blank"
                                rel="noreferrer"
                            >
                                <LuMessageCircle />
                            </a>
                            <a
                                href={`/t/${topic?.topic_id}`}
                                className="button aspect-square flex gap-1 items-center"
                                rel="noreferrer"
                                onClick={(event) => {
                                    event.preventDefault();
                                    navigator.clipboard.writeText(
                                        `https://ethereum.forum/t/${topic?.topic_id}`
                                    );
                                    toast.success('Link copied to clipboard');
                                }}
                            >
                                <LuLink />
                            </a>
                        </div>
                    </div>
                    <div className="flex items-center gap-2 justify-end">
                        <div className="flex items-center gap-1">
                            <FiEye />
                            {formatBigNumber(topic?.view_count ?? 0)}
                        </div>
                        <div className="flex items-center gap-1">
                            <FiHeart />
                            {formatBigNumber(topic?.like_count ?? 0)}
                        </div>
                        <div className="flex items-center gap-1">
                            <FiMessageSquare />
                            {formatBigNumber(topic?.post_count ?? 0)}
                        </div>
                    </div>
                </div>
                <div className="space-y-8 pb-10">
                    {status === 'pending' ? (
                        <div>Loading...</div>
                    ) : status === 'error' ? (
                        <div>Error fetching posts</div>
                    ) : (
                        <>
                            {data.pages?.map((page, i) => (
                                <Fragment key={i}>
                                    {page.posts?.map((post) => (
                                        <TopicPost key={post.post_id} post={post} />
                                    ))}
                                </Fragment>
                            ))}
                            <div className="mt-4 flex justify-center">
                                <button
                                    onClick={() => fetchNextPage()}
                                    disabled={!hasNextPage || isFetchingNextPage}
                                    className="px-4 py-2 bg-blue-500 text-white rounded disabled:opacity-50"
                                >
                                    {isFetchingNextPage
                                        ? 'Loading more...'
                                        : hasNextPage
                                          ? 'Load More'
                                          : 'No more posts'}
                                </button>
                            </div>
                        </>
                    )}
                </div>
            </div>
        </>
    );
}

const RelevantLink = ({ link }: { link: RelevantLink }) => {
    let icon = undefined;
    const url = link.url.toLowerCase();

    if (link.internal) {
        icon = <LuLink />;
    } else if (link.attachment) {
        icon = <LuPaperclip />;
    } else if (link.reflection) {
        icon = <LuMessageCircle />;
    } else if (isStandardsLink(url)) {
        icon = <SiEthereum />;
    } else if (isGithub(url)) {
        icon = <LuGithub />;
    } else if (url.startsWith('https://etherscan.io/')) {
        icon = <PiReceipt />;
    } else if (url.startsWith('https://www.youtube.com/')) {
        icon = <LuYoutube />;
    } else if (url.startsWith('https://www.reddit.com/')) {
        icon = <SiReddit />;
    } else if (isHackmd(url)) {
        icon = <LuNotebook />;
    } else if (url.startsWith('https://chatgpt.com/')) {
        icon = <SiOpenai />;
    }

    return (
        <a
            href={link.url}
            target="_blank"
            rel="noreferrer"
            className="flex justify-between hover:bg-secondary px-1.5 gap-3 items-center"
        >
            <div className="w-full flex items-center gap-1 truncate">
                {icon && <div className="size-4 text-sm">{icon}</div>}
                <div className="truncate">{link.title || link.root_domain}</div>
            </div>
            <div className="text-xs">{formatBigNumber(link.clicks)}</div>
        </a>
    );
};

const RefreshTopicButton = ({ discourseId, topicId }: { discourseId: string; topicId: string }) => {
    const { mutate: refreshTopic, isPending } = useTopicRefresh(discourseId, topicId);

    return (
        <button
            className="text-sm text-gray-500 hover:bg-secondary p-1 group"
            onClick={() => refreshTopic()}
        >
            <LuRefreshCcw
                className={classNames(
                    'transition-transform duration-200 group-active:animate-spin',
                    isPending && 'animate-spin'
                )}
            />
        </button>
    );
};

export const SignalIndicator = ({ date }: { date: Date }) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const diffHours = diff / (1000 * 60 * 60);

    return (
        <div className="text-lg text-primary/30 flex items-center relative w-fit">
            <LuSignal />
            <div className="absolute inset-0 text-primary flex items-center justify-center">
                {/* indicator */}
                {diffHours < 24 * 3 ? (
                    <LuSignalHigh />
                ) : diffHours < 24 * 7 ? (
                    <LuSignalMedium />
                ) : diffHours < 24 * 30 ? (
                    <LuSignalLow />
                ) : (
                    <LuSignalZero />
                )}
            </div>
        </div>
    );
};
