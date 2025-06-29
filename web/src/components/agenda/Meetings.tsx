import { Link } from '@tanstack/react-router';
import { addMinutes, format, isWithinInterval, parseISO } from 'date-fns';
import { FC, useMemo } from 'react';
import { SiGithub, SiGooglemeet, SiYoutube, SiZoom } from 'react-icons/si';

import { CalendarEvent } from '@/api/events';
import { convertYoutubeUrlToThumbnailUrl, getOccurence } from '@/routes/pm/$issueId';

import { TimeAgo } from '../TimeAgo';
import { CalendarDays } from './CalendarDays';

export const Meetings: FC<{ data: CalendarEvent[] }> = ({ data }) => <CalendarDays data={data} />;

const platformIcons = {
    Zoom: <SiZoom className="text-2xl" />,
    Google: <SiGooglemeet className="text-lg" />,
    Youtube: <SiYoutube className="text-lg" />,
} as const;

const isMeetingIn6Hours = (startTime?: string) => {
    return useMemo(() => {
        if (!startTime) return false;

        const start = parseISO(startTime);

        return isWithinInterval(new Date(), {
            start,
            end: addMinutes(start, 30),
        });
    }, [startTime]);
};
// testing live buttons
// const isMeetingIn6Hours = (start: string) => {
//     return useMemo(() => {
//         if (!start) return false;

//         const now = new Date();

//         return isWithinInterval(new Date(), {
//             start: now,
//             end: addMinutes(now, 30),
//         });
//     }, [start]);
// };

const MeetingStatus = ({ event }: { event: CalendarEvent }) => {
    const isLive = isMeetingIn6Hours(event.start);

    if (!event.start) return null;

    return (
        <div className="pb-2">
            {isLive ? (
                <div className="font-bold text-sm text-orange-700 animate-pulse flex items-center gap-2">
                    <span className="h-2 w-2 rounded-full bg-orange-700" />
                    <span>LIVE</span>
                </div>
            ) : (
                <div className="flex items-center">
                    <span className="text-secondary text-base">
                        <TimeAgo date={parseISO(event.start)} />
                    </span>
                </div>
            )}
        </div>
    );
};

const CompactMeetingButtons: FC<{ event: CalendarEvent; isLive: boolean }> = ({
    event,
    isLive,
}) => {
    if (isLive) return null;

    return (
        <div className="flex gap-2">
            {event.meetings.map((meeting) => (
                <Link
                    key={meeting.link}
                    to={meeting.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="button w-10 h-8 flex items-center justify-center"
                    title={`Join ${meeting.type} meeting`}
                >
                    <span className={meeting.type === 'Zoom' ? 'scale-120' : 'scale-100'}>
                        {platformIcons[meeting.type]}
                    </span>
                </Link>
            ))}
            {event.pm_number && (
                <Link
                    className="button w-10 h-8 flex items-center justify-center"
                    to="/pm/$issueId"
                    params={{ issueId: event.pm_number.toString() }}
                    title={`View PM #${event.pm_number}`}
                >
                    <SiGithub className="text-lg" />
                </Link>
            )}
        </div>
    );
};

export const MeetingPreview = ({ event }: { event: CalendarEvent }) => {
    const occurrence = getOccurence(event.pm_data, event.pm_number);
    const youtubeStream = occurrence?.youtube_streams?.[0]?.stream_url;
    const isLive = isMeetingIn6Hours(event.start);

    return (
        <div className="card gap-4 flex flex-col">
            <div className="flex justify-between items-end border">
                <MeetingStatus event={event} />

                <CompactMeetingButtons event={event} isLive={isLive} />
            </div>

            <div className="flex gap-4 justify-between">
                <div className="space-y-4">
                    <h3 className="font-bold truncate flex-1" title={event.summary}>
                        {event.summary}
                    </h3>
                    <p
                        dangerouslySetInnerHTML={{ __html: event.description ?? '' }}
                        className="grow max-h-40 overflow-y-auto"
                    />
                </div>

                {youtubeStream && (
                    <img
                        src={convertYoutubeUrlToThumbnailUrl(youtubeStream)}
                        alt="YouTube Stream"
                        className="rounded w-40 aspect-video object-cover"
                    />
                )}
            </div>

            {isLive && event.meetings.length > 0 && (
                <div className="flex gap-2">
                    {event.meetings.map((meeting) => (
                        <Link
                            key={meeting.link}
                            to={meeting.link}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="flex items-center justify-center gap-2 button w-fit"
                        >
                            <span className="text-xl border-r border-primary pr-2">
                                {platformIcons[meeting.type]}
                            </span>
                            <span className="text-sm">Join Meeting </span>
                        </Link>
                    ))}
                </div>
            )}
        </div>
    );
};
