import { format, parseISO } from 'date-fns';
import { FC, useState } from 'react';
import { LuExternalLink, LuPlay } from 'react-icons/lu';

import { CalendarEvent, useEventsRecent } from '@/api/events';

export const getYoutubeVideoId = (url: string) => {
    try {
        const parsedUrl = new URL(url);
        const hostname = parsedUrl.hostname.replace('www.', '');

        if (hostname === 'youtu.be') return parsedUrl.pathname.slice(1).split('/')[0] || null;

        if (hostname.endsWith('youtube.com')) {
            return (
                parsedUrl.searchParams.get('v') ||
                parsedUrl.pathname.match(/\/(?:embed|live|shorts)\/([^/]+)/)?.[1] ||
                null
            );
        }
    } catch {
        return null;
    }

    return null;
};

const getYoutubeThumbnailUrl = (videoId: string) =>
    `https://i.ytimg.com/vi/${videoId}/hqdefault.jpg`;

type Video = {
    videoId: string;
    url: string;
    title: string;
    start?: string;
    pmNumber?: number;
};

const getEventVideos = (event: CalendarEvent): Video[] => {
    const meetingVideos = event.meetings
        .filter((meeting) => meeting.type === 'Youtube')
        .map((meeting) => meeting.link);
    const occurrenceVideos =
        event.pm_data && 'occurrences' in event.pm_data
            ? (event.pm_data.occurrences ?? []).flatMap(
                  (occurrence) =>
                      occurrence.youtube_streams?.map((stream) => stream.stream_url) ?? []
              )
            : [];

    return [...meetingVideos, ...occurrenceVideos].flatMap((url) => {
        if (!url) return [];

        const videoId = getYoutubeVideoId(url);

        return videoId
            ? [
                  {
                      videoId,
                      url,
                      title: event.summary || 'Protocol meeting',
                      start: event.start,
                      pmNumber: event.pm_number,
                  },
              ]
            : [];
    });
};

const VideoPreview: FC<{ video: Video }> = ({ video }) => {
    const [hasThumbnail, setHasThumbnail] = useState(true);

    return (
        <div className="relative flex aspect-video items-center justify-center bg-secondary">
            {hasThumbnail && (
                <img
                    src={getYoutubeThumbnailUrl(video.videoId)}
                    alt=""
                    className="size-full object-cover"
                    loading="lazy"
                    onError={() => setHasThumbnail(false)}
                />
            )}
            <span className="absolute flex items-center gap-2 rounded-full border border-primary/50 bg-primary/90 px-3 py-1.5 text-sm text-primary transition-colors group-hover:bg-tertiary">
                <LuPlay className="size-4 text-secondary" />
                Watch recording
            </span>
        </div>
    );
};

const getMeetingDetails = (video: Video) => {
    const details = [
        video.start ? format(parseISO(video.start), 'MMM d, yyyy HH:mm') : null,
        video.pmNumber ? `PM #${video.pmNumber}` : null,
    ].filter((detail): detail is string => Boolean(detail));

    return details.join(' · ');
};

export const AgendaVideos: FC = () => {
    const { data: recent } = useEventsRecent();
    const allVideos = Array.from(
        new Map(
            (recent ?? []).flatMap(getEventVideos).map((video) => [video.videoId, video])
        ).values()
    );
    const videos = allVideos.slice(0, 24);

    return (
        <div className="space-y-3">
            <div className="flex items-baseline justify-between gap-3">
                <div>
                    <h2 className="text-lg font-bold">Recordings</h2>
                    <p className="text-sm text-primary/70">Recent protocol meeting videos</p>
                </div>
                <span className="text-sm text-primary/70">
                    {videos.length} of {allVideos.length} videos
                </span>
            </div>
            {videos.length === 0 ? (
                <div className="card text-primary/70">No recent recordings are available.</div>
            ) : (
                <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    {videos.map((video) => (
                        <li key={video.videoId} className="card no-padding overflow-hidden">
                            <a
                                href={video.url}
                                target="_blank"
                                rel="noreferrer"
                                className="group block"
                            >
                                <VideoPreview video={video} />
                                <div className="flex items-start justify-between gap-2 p-3">
                                    <div className="space-y-1">
                                        <h3 className="font-bold">{video.title}</h3>
                                        <p className="text-sm text-primary/70">
                                            {getMeetingDetails(video)}
                                        </p>
                                    </div>
                                    <LuExternalLink className="mt-0.5 shrink-0 text-primary/70" />
                                </div>
                            </a>
                        </li>
                    ))}
                </ul>
            )}
        </div>
    );
};
