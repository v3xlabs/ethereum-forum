import { differenceInMinutes, parseISO } from 'date-fns';
import { LuCalendar } from 'react-icons/lu';
import { SiZoom } from 'react-icons/si';

import { useEventsUpcoming } from '@/api/events';

import { CommandGroup, CommandItem, CommandSeparator } from '../Command';

export const UpcomingCalendarEvent = () => {
    const { data } = useEventsUpcoming();

    if (!data || data.length === 0) return null;

    // Find the soonest event
    const [soonest] = data
        .filter((e) => e.start)
        .sort((a, b) => new Date(a.start!).getTime() - new Date(b.start!).getTime());

    if (!soonest) return null;

    const startDate = parseISO(soonest.start!);
    const now = new Date();
    const minutes = Math.max(0, differenceInMinutes(startDate, now));
    const hasStarted = now >= startDate;

    if (minutes > 10) return null;

    // Find the first Zoom meeting link if available
    const zoomMeeting = soonest.meetings?.find((m) => m.type === 'Zoom');

    return (
        <>
            <CommandGroup>
                <div className="flex items-center gap-2 my-2 text-base font-semibold">
                    <LuCalendar className="size-5" />
                    Upcoming
                </div>
                <CommandItem className="flex items-center gap-4 p-4 rounded-lg border border-primary bg-blue-50/80 shadow-md">
                    <div className="flex flex-col flex-1 min-w-0">
                        <span className="font-bold text-base truncate">
                            {soonest.summary || 'Untitled Event'}
                        </span>
                        <span className="block text-xs text-muted-foreground">
                            {hasStarted
                                ? 'in progress'
                                : minutes === 0
                                  ? 'less than a minute'
                                  : `${minutes} minute${minutes !== 1 ? 's' : ''}`}
                        </span>
                    </div>
                    <span className="ml-auto flex items-center">
                        {zoomMeeting && (
                            <a
                                href={zoomMeeting.link}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="flex items-center gap-1 px-3 py-1 rounded bg-blue-600 text-white font-semibold hover:bg-blue-700 transition text-sm shadow"
                            >
                                <SiZoom className="mr-1" /> Join
                            </a>
                        )}
                    </span>
                </CommandItem>
            </CommandGroup>
            <CommandSeparator />
        </>
    );
};
