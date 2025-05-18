import { createFileRoute } from '@tanstack/react-router';
import classNames from 'classnames';
import { parseISO } from 'date-fns';
import { useEffect } from 'react';
import { Fragment } from 'react/jsx-runtime';
import { FiEye, FiHeart, FiMessageSquare } from 'react-icons/fi';
import {
    LuArrowDown,
    LuArrowUp,
    LuGithub,
    LuLink,
    LuMessageCircle,
    LuNotebook,
    LuPaperclip,
    LuRefreshCcw,
    LuYoutube,
} from 'react-icons/lu';
import { PiReceipt } from 'react-icons/pi';
import { SiEthereum, SiOpenai, SiReddit } from 'react-icons/si';

import { getTopic, usePostsInfinite, useTopic, useTopicRefresh } from '@/api/topics';
import { ExpandableList } from '@/components/list/ExpandableList';
import { TimeAgo } from '@/components/TimeAgo';
import { TopicPost } from '@/components/topic/TopicPost';
import { decodeCategory } from '@/util/category';
import { isGithub, isHackmd, isStandardsLink, spliceRelatedLinks } from '@/util/links';
import { formatBigNumber } from '@/util/numbers';
import { queryClient } from '@/util/query';

type DiscourseUser = {
    id: number;
    // stylistic
    name: string;
    // real
    username: string;
    avatar_template: string;
};

type TopicDetails = {
    links?: Partial<RelevantLink>[];
    created_by?: DiscourseUser;
};

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

const normalizeRelevantLink = (link: Partial<RelevantLink>): RelevantLink => {
    return {
        ...link,
        domain: link.domain ?? '',
        root_domain: link.root_domain ?? '',
        url: link.url ?? '',
        title: link.title ?? '',
        internal: link.internal ?? false,
        attachment: link.attachment ?? false,
        reflection: link.reflection ?? false,
        clicks: link.clicks ?? 0,
    };
};

const RouteComponent = () => {
    const { topicId } = Route.useParams();
    const { data: topic } = useTopic(topicId);

    const { data, fetchNextPage, hasNextPage, isFetchingNextPage, status } =
        usePostsInfinite(topicId);

    const extra = topic?.extra as Record<string, unknown>;
    const tags = decodeCategory(extra?.['category_id'] as number);

    const details = extra?.details as TopicDetails | undefined;
    const all_links = (details?.links || [])
        .map(normalizeRelevantLink)
        .sort((a, b) => b.clicks - a.clicks);
    const [standards_links, relevant_links1] = spliceRelatedLinks(all_links, (link) =>
        isStandardsLink(link.url)
    );
    const [github_links, relevant_links] = spliceRelatedLinks(relevant_links1, (link) =>
        isGithub(link.url)
    );
    const creator = details?.created_by as DiscourseUser;

    useEffect(() => {
        document.documentElement.classList.add('prose-page');

        return () => {
            document.documentElement.classList.remove('prose-page');
        };
    }, []);

    return (
        <>
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
                        <li className="flex items-center gap-1 px-1.5 justify-between">
                            <div className="text-base">Last Post</div>
                            <div className="text-base">
                                {topic?.last_post_at && (
                                    <TimeAgo date={parseISO(topic.last_post_at)} />
                                )}
                            </div>
                        </li>
                        <li className="flex items-center gap-1 px-1.5 justify-between">
                            <div className="text-base">Source</div>
                            <div className="text-base flex items-center">
                                <a
                                    href={'https://ethereum-magicians.org/t/' + topic?.topic_id}
                                    target="_blank"
                                    rel="noreferrer"
                                    className="hover:underline"
                                >
                                    ethmag/{topic?.topic_id}
                                </a>
                                <RefreshTopicButton topicId={topicId} />
                            </div>
                        </li>
                    </ul>
                </div>
                {standards_links.length > 0 && (
                    <ExpandableList title="Standards Links" maxItems={4}>
                        {standards_links.map((link) => (
                            <li key={link.url}>
                                <RelevantLink link={normalizeRelevantLink(link)} />
                            </li>
                        ))}
                    </ExpandableList>
                )}
                {github_links.length > 0 && (
                    <ExpandableList title="Github Links" maxItems={4}>
                        {github_links.map((link) => (
                            <li key={link.url}>
                                <RelevantLink link={normalizeRelevantLink(link)} />
                            </li>
                        ))}
                    </ExpandableList>
                )}
                {relevant_links.length > 0 && (
                    <ExpandableList title="Related Links" maxItems={4}>
                        {relevant_links?.map((link) => (
                            <li key={link.url}>
                                <RelevantLink link={normalizeRelevantLink(link)} />
                            </li>
                        ))}
                    </ExpandableList>
                )}
            </div>
            <div className="mx-auto w-full prose-width pt-8 px-2 space-y-4 relative">
                <UpDownScroller />
                <div>
                    <h1 className="text-2xl">
                        <b>{topic?.title}</b>
                    </h1>
                    <div className="flex items-center gap-2">
                        {tags?.map((tag) => (
                            <div
                                key={tag}
                                className="text-sm text-gray-500 bg-primary px-1 border border-primary"
                            >
                                {tag}
                            </div>
                        ))}
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
};

export const Route = createFileRoute('/t/$topicId/')({
    component: RouteComponent,
    beforeLoad: async ({ params }) => {
        const topic = await queryClient.ensureQueryData(getTopic(params.topicId));

        return {
            title: topic?.title,
        };
    },
});
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

const RefreshTopicButton = ({ topicId }: { topicId: string }) => {
    const { mutate: refreshTopic, isPending } = useTopicRefresh(topicId);

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

const UpDownScroller = () => {
    return (
        <div className="items-center gap-2 absolute right-0 top-28 hidden md:flex">
            <div className="fixed flex flex-col gap-2 items-center translate-x-full ">
                <button
                    className="text-sm hover:bg-secondary p-1 group border border-primary rounded-md"
                    onClick={() => {
                        window.scrollTo({
                            top: 0,
                            behavior: 'smooth',
                        });
                    }}
                >
                    <LuArrowUp />
                </button>
                <button
                    className="text-sm hover:bg-secondary p-1 group border border-primary rounded-md"
                    onClick={() => {
                        window.scrollTo({
                            top: document.body.scrollHeight,
                            behavior: 'smooth',
                        });
                    }}
                >
                    <LuArrowDown />
                </button>
            </div>
        </div>
    );
};
