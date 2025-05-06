import { Link } from '@tanstack/react-router';
import { format, parseISO, isSameDay } from 'date-fns';
import { FC } from 'react';
import { FiRefreshCcw } from 'react-icons/fi';
import { SiGooglemeet, SiYoutube, SiZoom } from 'react-icons/si';

import { CalendarEvent, Meeting } from '@/api/events';
import { convertYoutubeUrlToThumbnailUrl, getOccurence } from '@/routes/pm/$issueId';

import { TimeAgo } from '../TimeAgo';

export const Meetings: FC<{ data: CalendarEvent[] }> = ({ data }) => {
    return (
        <div className="flex gap-3">
            <div className="flex flex-col">
                {data?.map((event, idx) => {
                    const prevEvent = idx > 0 ? data[idx - 1] : undefined;
                    const currentDate = event.start ? parseISO(event.start) : undefined;
                    const prevDate = prevEvent?.start ? parseISO(prevEvent.start) : undefined;
                    const showDateSeparator =
                        prevDate && currentDate && !isSameDay(prevDate, currentDate);
                    const key = `${event?.uid ?? idx}_${event?.start ?? ''}`;
                    return (
                        <>
                            {showDateSeparator && (
                                <div className="text-center my-2 text-xs font-semibold text-primary/80">
                                    {currentDate ? format(currentDate, 'EEEE, MMMM d, yyyy') : ''}
                                </div>
                            )}
                            {/* @ts-ignore */}
                            <MeetingPreview key={key} event={event} />
                        </>
                    );
                })}
            </div>
        </div>
    );
};

export const platformToIcon = {
    Zoom: <SiZoom />,
    Google: <SiGooglemeet />,
    Youtube: <SiYoutube />,
};

export const MeetingLink = ({ meeting }: { meeting: Meeting }) => {
    return (
        <a
            href={meeting.link}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 button"
        >
            {platformToIcon[meeting.type]}
            <span className="text-sm">{meeting.type}</span>
        </a>
    );
};

export const DebugRichData = ({ event }: { event: unknown }) => {
    return (
        <div className="border border-primary px-1 whitespace-pre-wrap">
            {JSON.stringify(event, null, 2)}
        </div>
    );
};

export const MeetingPreview = ({ event }: { event: CalendarEvent }) => {
    const occurence = getOccurence(event.pm_data, event.pm_number);

    return (
        <div className="flex gap-2">
            <div className="border-r w-full border-primary pr-2 max-w-24 break-all text-right">
                <div className="text-sm">
                    {event?.start && format(parseISO(event?.start), 'HH:mm')}
                </div>
                <div className="text-xs">
                    (<TimeAgo date={parseISO(event.start)} />)
                </div>
            </div>
            <div className="card flex gap-2 grow mb-4">
                {occurence &&
                    'youtube_streams' in occurence &&
                    occurence.youtube_streams?.[0]?.stream_url && (
                        <div className="w-full aspect-video max-w-40 flex items-center">
                            <img
                                src={convertYoutubeUrlToThumbnailUrl(
                                    occurence.youtube_streams?.[0].stream_url
                                )}
                                alt="Youtube Stream"
                                className="w-full aspect-video"
                            />
                        </div>
                    )}
                <div className="flex flex-col gap-2 grow">
                    <div className="flex items-center gap-2 justify-between">
                        <h3 className="font-bold">{event.summary}</h3>
                        <div className="flex items-center gap-2">
                            {event.pm_number && (
                                <Link
                                    to={'/pm/$issueId'}
                                    params={{ issueId: event.pm_number.toString() }}
                                    className="button"
                                >
                                    #{event.pm_number}
                                </Link>
                            )}
                            {event.occurance == 'Recurring' && <FiRefreshCcw className="size-3" />}
                        </div>
                    </div>
                    <div
                        dangerouslySetInnerHTML={{ __html: event.description ?? '' }}
                        className="grow max-h-40 overflow-y-auto"
                    />
                    <div className="flex justify-between flex-wrap">
                        <div className="flex flex-wrap gap-2">
                            {event.meetings.map((meeting) => (
                                <MeetingLink key={meeting.link} meeting={meeting} />
                            ))}
                        </div>
                        <p className="text-sm text-gray-500 text-end flex items-end">
                            {event.start && <TimeAgo date={parseISO(event.start)} />}
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
};
