import { Link } from '@tanstack/react-router';
import { addHours, addMinutes, format, isAfter, parseISO } from 'date-fns';
import { FiDroplet } from 'react-icons/fi';

import { useEventsUpcoming } from '@/api/events';

import { TimeAgo } from '../TimeAgo';
import { platformToIcon } from './Meetings';

// Calendar related types and components
type CalendarEvent = {
    id: string;
    title: string;
    description?: string;
    startTime: string;
    duration: number;
    color?: string;
};

export const ProtocolAgendaUpcoming = () => {
    const { data } = useEventsUpcoming();

    // Separate events within next 24 hours and beyond
    const now = new Date();
    const twelveHoursLater = addHours(now, 24);

    const upcomingEvents = data?.filter((event) => {
        const startDate = event.start ? parseISO(event.start) : null;

        return startDate && isAfter(startDate, now) && !isAfter(startDate, twelveHoursLater);
    });

    const laterEvents = data?.filter((event) => {
        const startDate = event.start ? parseISO(event.start) : null;

        return startDate && isAfter(startDate, twelveHoursLater);
    });

    // Format events for calendar
    const calendarEvents: CalendarEvent[] =
        upcomingEvents?.map((event) => ({
            id: event.uid || `event-${event.summary}`,
            title: event.summary || 'Untitled Event',
            description: event.meetings.map((m) => m.type).join(', '),
            startTime: event.start || new Date().toISOString(),
            // Assume 30 minutes duration for each event
            duration: 30,
            color: 'rgb(var(--theme-bg-primary))',
        })) || [];

    const showCalendar = calendarEvents.length > 0;

    return (
        <div className="space-y-1.5">
            <div className="px-1.5">
                <h3 className="font-bold w-full border-b border-b-primary pb-1">Protocol Agenda</h3>
            </div>

            {showCalendar && (
                <div className="border rounded-lg overflow-hidden bg-background mb-4 border-primary">
                    <div className="p-3 border-b border-b-primary">
                        <h2 className="text-base font-semibold">Today ({calendarEvents.length})</h2>
                    </div>

                    <div className="flex h-[420px] overflow-y-auto">
                        {/* Time column */}
                        <div className="w-16 flex-shrink-0 border-r border-primary bg-secondary h-fit">
                            {Array.from({ length: 24 }, (_, i) => {
                                const hour = (i + now.getHours()) % 24;

                                return (
                                    <div
                                        key={i}
                                        className="h-16 px-2 flex items-start justify-end text-xs text-primary"
                                    >
                                        <span className="mt-1">
                                            {format(new Date().setHours(hour, 0, 0, 0), 'h:mm a')}
                                        </span>
                                    </div>
                                );
                            })}
                        </div>

                        {/* Events column */}
                        <div className="flex-1 relative">
                            {Array.from({ length: 24 }, (_, i) => (
                                <div key={i} className="h-16 border-b border-secondary" />
                            ))}

                            {/* Render events */}
                            {calendarEvents.map((event) => {
                                const startDate = parseISO(event.startTime);
                                const startHour = startDate.getHours();
                                const startMinute = startDate.getMinutes();
                                const currentHour = now.getHours();

                                // Calculate position relative to current time
                                const hoursFromNow = (startHour - currentHour + 24) % 24;
                                const startFromTop = (hoursFromNow * 60 + startMinute) * (64 / 60); // 64px per hour
                                const height = Math.max(32, event.duration * (64 / 60)); // Minimum height

                                return (
                                    <div
                                        key={event.id}
                                        className="absolute rounded-md p-2 overflow-hidden hover:overflow-visible hover:!h-fit border border-primary shadow-sm"
                                        style={{
                                            top: `${startFromTop}px`,
                                            height: `${height}px`,
                                            width: '90%',
                                            left: '5%',
                                            backgroundColor:
                                                event.color || 'rgb(var(--theme-bg-secondary))',
                                        }}
                                    >
                                        <div className="font-medium truncate text-sm">
                                            {event.title}
                                        </div>
                                        <div className="text-xs opacity-90">
                                            {format(startDate, 'h:mm a')} -{' '}
                                            {format(
                                                addMinutes(startDate, event.duration),
                                                'h:mm a'
                                            )}
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                </div>
            )}

            <ul>
                {laterEvents?.map((event) => {
                    const issueId = event.pm_number;

                    return (
                        <li key={event.uid}>
                            <Link
                                to={issueId ? '/pm/$issueId' : '/c/$channelId'}
                                params={
                                    issueId
                                        ? { issueId: issueId.toString() }
                                        : { channelId: 'agenda' }
                                }
                                className="flex justify-between hover:bg-secondary px-1.5 gap-3"
                            >
                                <div className="flex items-center gap-1 truncate text-base">
                                    {issueId && <FiDroplet className="text-base size-4" />}
                                    <h4 className="truncate">{event.summary}</h4>
                                    {event.meetings.map((meeting) => {
                                        return platformToIcon[meeting.type];
                                    })}
                                </div>
                                <p className="text-sm flex-1 whitespace-nowrap text-gray-500 text-end">
                                    {event.start && <TimeAgo date={parseISO(event.start)} />}
                                </p>
                            </Link>
                        </li>
                    );
                })}
            </ul>
        </div>
    );
};
