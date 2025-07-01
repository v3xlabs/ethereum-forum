import { Link } from '@tanstack/react-router';
import { addHours, addMinutes, format, isWithinInterval, parseISO } from 'date-fns';
import parse from 'html-react-parser';
import { FC, useMemo } from 'react';
import { SiGithub, SiGooglemeet, SiYoutube, SiZoom } from 'react-icons/si';

import { CalendarEvent } from '@/api/events';
import { convertYoutubeUrlToThumbnailUrl, getOccurence } from '@/routes/pm/$issueId';

import { TimeAgo } from '../TimeAgo';
import { CalendarDays } from './CalendarDays';

export const Meetings: FC<{ data: CalendarEvent[] }> = ({ data }) => <CalendarDays data={data} />;

export const platformIcons = {
    Zoom: <SiZoom className="text-xl text-blue-600" />,
    Google: <SiGooglemeet className="text-md" />,
    Youtube: <SiYoutube className="text-lg text-red-500" />,
    Github: <SiGithub />,
};

const isMeetingLive = (startTime: string) => {
    return useMemo(() => {
        if (!startTime) return false;

        return isWithinInterval(new Date(), {
            start: startTime,
            end: addMinutes(startTime, 30),
            // testing live buttons
            // start: new Date(),
            // end: addMinutes(new Date(), 30),
        });
    }, [startTime]);
};

const isMeetingIn6Hours = (startTime: string) => {
    return useMemo(() => {
        if (!startTime) return false;

        const now = new Date();

        return isWithinInterval(startTime, {
            start: now,
            end: addHours(now, 6),
        });
    }, [startTime]);
};

const MeetingStatus = ({ event }: { event: CalendarEvent }) => {
    if (!event.start) return null;

    return (
        <div className="inline-block">
            {isMeetingLive(event.start) ? (
                <>
                    <span className="h-1.5 w-1.5 rounded-full bg-orange-400 animate-pulse" />
                    <span className="font-bold text-orange-400 text-md">LIVE</span>
                </>
            ) : (
                <span className="text-secondary">
                    <TimeAgo date={parseISO(event.start)} />
                </span>
            )}
            <span> - </span>
            <span className="text-primary/70">{format(event.start, 'HH:mm')}</span>
        </div>
    );
};

const CompactMeetingButtons: FC<{
    event: CalendarEvent;
    youtubeStream: string;
}> = ({ event, youtubeStream }) => {
    const showButtons =
        !!event.start && (isMeetingIn6Hours(event.start) || isMeetingLive(event.start));

    return (
        <div className="flex gap-2 justify-center">
            {!showButtons &&
                event.meetings.map((meeting) => (
                    <Link
                        key={meeting.link}
                        to={meeting.link}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="button aspect-square flex items-center justify-center button-ghost"
                    >
                        <span>{platformIcons[meeting.type]}</span>
                    </Link>
                ))}

            {!showButtons && youtubeStream && (
                <Link
                    to={youtubeStream}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="button aspect-square flex items-center justify-center button-ghost"
                >
                    {platformIcons.Youtube}
                </Link>
            )}

            {event.pm_number && (
                <a
                    className="button aspect-square flex items-center justify-center button-ghost"
                    href={`https://github.com/ethereum/pm/issues/${event.pm_number}`}
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    {platformIcons.Github}
                </a>
            )}
        </div>
    );
};

const ExpandedMeetingButtons: FC<{
    event: CalendarEvent;
    youtubeStream: string;
}> = ({ event, youtubeStream }) => {
    const showButtons =
        !!event.start && (isMeetingIn6Hours(event.start) || isMeetingLive(event.start));

    if (!showButtons) return <></>;

    return (
        <div className="flex gap-2">
            {event.meetings.map((meeting) => (
                <Link
                    key={meeting.link}
                    to={meeting.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center justify-center gap-2 button px-3"
                >
                    <span className="text-xl border-r border-secondary pr-2">
                        {platformIcons[meeting.type]}
                    </span>
                    <span className="text-sm">Participate</span>
                </Link>
            ))}

            {youtubeStream && (
                <Link
                    to={youtubeStream}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center justify-center gap-2 button w-fit"
                >
                    <span className="text-xl border-r border-primary pr-2">
                        {platformIcons.Youtube}
                    </span>
                    <span className="text-sm">Watch</span>
                </Link>
            )}
        </div>
    );
};

export const MeetingCard = ({ event }: { event: CalendarEvent }) => {
    const occurrence = getOccurence(event.pm_data, event.pm_number);
    const youtubeStream = occurrence?.youtube_streams?.[0]?.stream_url;

    const cardContent = (
        <>
            <div className="flex justify-between items-start h-4">
                <MeetingStatus event={event} />
                <CompactMeetingButtons event={event} youtubeStream={youtubeStream} />
            </div>

            <div className="flex gap-2">
                {/* yt link should be embedded */}
                {youtubeStream && (
                    <Link to={youtubeStream}>
                        <img
                            src={convertYoutubeUrlToThumbnailUrl(youtubeStream)}
                            alt="YouTube Stream"
                            className="w-60 aspect-video object-cover rounded"
                        />
                    </Link>
                )}
                <div className="space-y-1">
                    <h3 className="font-bold">{event.summary}</h3>
                    {event.description && <p>{parse(event.description)}</p>}
                </div>
            </div>

            <ExpandedMeetingButtons event={event} youtubeStream={youtubeStream} />
        </>
    );

    return event.pm_number ? (
        <Link
            to="/pm/$issueId"
            params={{ issueId: event.pm_number?.toString() }}
            target="_blank"
            rel="noopener noreferrer"
            title="View on GitHub"
            className="card gap-2 flex flex-col justify-center space-y-1 pointer"
        >
            {cardContent}
        </Link>
    ) : (
        <div className="card gap-2 flex flex-col justify-center space-y-1">{cardContent}</div>
    );
};

export const DebugRichData = ({ event }: { event: unknown }) => {
    return (
        <div className="border border-primary px-1 whitespace-pre-wrap">
            {JSON.stringify(event, null, 2)}
        </div>
    );
};
