import 'react-lite-youtube-embed/dist/LiteYouTubeEmbed.css';

import { createFileRoute, Link } from '@tanstack/react-router';
import { formatDistance } from 'date-fns';
import { FiGithub } from 'react-icons/fi';
import { SiDiscourse } from 'react-icons/si';
import LiteYouTubeEmbed from 'react-lite-youtube-embed';
import sanitizeHtml from 'sanitize-html';

import { getPM, PMMeetingData, usePM } from '@/api/pm';
import { components } from '@/api/schema.gen';
import { usePosts } from '@/api/topics';
import { queryClient } from '@/util/query';

export const Route = createFileRoute('/pm/$issueId/')({
    component: RouteComponent,
    beforeLoad: async ({ params }) => {
        const { issueId } = params;
        const pm = await queryClient.ensureQueryData(getPM(Number(params.issueId)));
        const occurence = getOccurence(pm as any, Number(issueId));

        return {
            title: occurence?.issue_title,
        };
    },
});

type OneOffMeeting = components['schemas']['PMOneOffMeeting'];
type Occurrence = components['schemas']['PMOccurrence'];

function extractAgendaHtml(html: string) {
    const agendaHeaderMatch = html.match(/<h([1-6])>.*?>.*?Agenda.*?<\/h\1>/i);

    if (!agendaHeaderMatch) return '';

    const [agendaHeader] = agendaHeaderMatch;
    const agendaHeaderIndex = html.indexOf(agendaHeader);

    const ulStart = html.indexOf('<ul>', agendaHeaderIndex);
    const ulEnd = html.indexOf('</ul>', ulStart);

    if (ulStart === -1 || ulEnd === -1) return '';

    const agendaList = html.slice(ulStart, ulEnd + 5); // </ul> is exactly 5 chars

    return agendaList;
}

function RouteComponent() {
    const { issueId } = Route.useParams();
    const { data: pm } = usePM(Number(issueId));
    const occurence = getOccurence(pm as any, Number(issueId));
    const { data: post } = usePosts(occurence.discourse_topic_id || '', 1);

    // Parse post content to extract more youtube links
    post?.posts.forEach((post) => {
        // Example: "<p>YouTube recording available: <a href="https://youtu.be/dotZwMwz_8Q">https://youtu.be/dotZwMwz_8Q</a></p>"
        // Allow youtu.be, youtube.com, and www.youtube.com links
        const youtubeLinks = post.cooked.match(
            /<a href="(https?:\/\/)?(www\.)?(youtube\.com|youtu\.?be)\/.+?<\/a>/g
        );

        if (youtubeLinks) {
            youtubeLinks.forEach((link) => {
                const url = link.match(/href="([^"]+)"/)?.[1];

                // Insert into occurence.youtube_streams if it doesn't exist
                if (
                    url &&
                    typeof occurence === 'object' &&
                    occurence !== null &&
                    'youtube_streams' in occurence
                ) {
                    const videoId = parseYoutubeUrl(url);

                    if (videoId) {
                        occurence.youtube_streams = occurence.youtube_streams || [];

                        if (
                            !occurence.youtube_streams.find(
                                (stream: any) => stream.stream_url === url
                            )
                        ) {
                            // @ts-expect-error: dynamic property check above
                            occurence.youtube_streams.push({
                                stream_url: url,
                            });
                        }
                    }
                }
            });
        }
    });

    const agenda = (
        <div
            dangerouslySetInnerHTML={{
                __html: sanitizeHtml(extractAgendaHtml(post?.posts[0].cooked || ''), {
                    allowedTags: ['ul', 'li', 'a'],
                    allowedAttributes: {
                        ul: ['style'],
                        a: ['href', 'style'],
                    },
                    transformTags: {
                        ul: sanitizeHtml.simpleTransform('ul', {
                            style: 'list-style-type: square',
                        }),
                        a: sanitizeHtml.simpleTransform('a', {
                            style: 'color: #1e90ff',
                        }),
                    },
                }),
            }}
        />
    );

    const inPast = new Date(occurence?.start_time as string) < new Date();

    if (!pm) {
        return (
            <div className="mx-auto w-full max-w-screen-lg pt-8 px-2 space-y-4">
                This PM event could not be found
            </div>
        );
    }

    return (
        <div className="mx-auto w-full max-w-screen-lg pt-8 px-2 space-y-4">
            <div>
                <h2 className="">
                    pm/<b>{issueId}</b>
                </h2>
                <h1 className="text-2xl font-bold">
                    {occurence?.issue_title || occurence?.issue_number}
                </h1>
            </div>
            <div className="flex flex-row gap-2">
                {occurence?.discourse_topic_id && (
                    <Link
                        to="/t/$topicId"
                        params={{ topicId: occurence.discourse_topic_id }}
                        className="button flex w-fit items-center gap-2"
                    >
                        <SiDiscourse />
                        Thread
                    </Link>
                )}
                <a
                    href={`https://github.com/ethereum/pm/issues/${occurence?.issue_number}`}
                    className="button flex w-fit items-center gap-2"
                >
                    <FiGithub /> Issue
                </a>
            </div>

            {'youtube_streams' in occurence && (occurence?.youtube_streams?.length || 0) > 0 && (
                <ul>
                    {occurence.youtube_streams?.map((stream) => (
                        <li key={stream.stream_url}>
                            <div className="rounded-lg overflow-hidden max-w-md w-full my-4">
                                <LiteYouTubeEmbed
                                    id={parseYoutubeUrl(stream.stream_url || '') || ''}
                                    title={occurence.issue_title || 'PM Meeting'} // For accessibility, never shown
                                />
                            </div>
                        </li>
                    ))}
                </ul>
            )}

            <p>
                This {'occurrence_rate' in pm ? pm.occurrence_rate : ''}
                {'is_recurring' in pm && pm.is_recurring ? ' recurring ' : ''} event start
                {inPast ? 'ed' : 's'}{' '}
                {formatDistance(occurence?.start_time as string, new Date(), {
                    addSuffix: true,
                })}
            </p>

            <h2 className="text-xl">Agenda</h2>
            {agenda}
        </div>
    );
}

export const getOccurence = (pm: PMMeetingData, issueId: number): OneOffMeeting | Occurrence => {
    if ('occurrences' in pm) {
        // @ts-ignore
        return pm.occurrences?.find((occurrence) => occurrence.issue_number === issueId) as
            | Occurrence
            | undefined;
    }

    return pm;
};

const parseYoutubeUrl = (url: string) => {
    if (!url) {
        return null;
    }

    if (url.includes('youtu.be/')) {
        return url.split('youtu.be/')[1];
    }

    if (url.includes('v=')) {
        return url.split('v=')[1];
    }

    return null;
};

// https://i3.ytimg.com/vi/YvlLhvICtbc/maxresdefault.jpg
export const convertYoutubeUrlToThumbnailUrl = (url: string) => {
    const videoId = parseYoutubeUrl(url);

    return `https://i3.ytimg.com/vi/${videoId}/maxresdefault.jpg`;
};
