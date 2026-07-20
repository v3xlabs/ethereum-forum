import { Link } from '@tanstack/react-router';
import {
    addDays,
    eachDayOfInterval,
    format,
    isSameDay,
    isToday,
    isWeekend,
    startOfDay,
    startOfWeek,
    subDays,
} from 'date-fns';
import { FC } from 'react';

import { CalendarEvent } from '@/api/events';

const weekdayLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

export const CalendarOverview: FC<{ data: CalendarEvent[] }> = ({ data }) => {
    const calendarStart = startOfWeek(subDays(startOfDay(new Date()), 7), { weekStartsOn: 1 });
    const calendarDays = eachDayOfInterval({
        start: calendarStart,
        end: addDays(calendarStart, 41),
    });
    const calendarEvents = Array.from(
        new Map(
            data
                .filter((event) => event.start)
                .map((event) => [`${event.uid}-${event.start}`, event])
        ).values()
    );
    const hasWeekendEvents = calendarEvents.some((event) => event.start && isWeekend(event.start));
    const visibleDays = hasWeekendEvents
        ? calendarDays
        : calendarDays.filter((day) => !isWeekend(day));
    const visibleWeekdayLabels = hasWeekendEvents ? weekdayLabels : weekdayLabels.slice(0, 5);

    return (
        <section className="space-y-3 overflow-x-auto" aria-labelledby="agenda-calendar-title">
            <div className="flex items-baseline justify-between gap-3">
                <div>
                    <h2 id="agenda-calendar-title" className="font-bold">
                        Calendar
                    </h2>
                    <p className="text-sm text-primary/70">
                        Last week and the next five weeks at a glance
                    </p>
                </div>
                <span className="shrink-0 text-sm text-primary/70">All times are local</span>
            </div>
            <div
                className={`grid min-w-[560px] border-l border-t border-primary/50 ${
                    hasWeekendEvents ? 'grid-cols-7' : 'grid-cols-5'
                }`}
            >
                {visibleWeekdayLabels.map((weekday) => (
                    <div
                        key={weekday}
                        className="border-b border-r border-primary/50 px-2 py-1 text-center text-xs text-primary/70"
                    >
                        {weekday}
                    </div>
                ))}
                {visibleDays.map((day) => {
                    const dayEvents = calendarEvents.filter(
                        (event) => event.start && isSameDay(event.start, day)
                    );

                    return (
                        <div
                            key={format(day, 'yyyy-MM-dd')}
                            className="min-h-24 border-b border-r border-primary/50 p-1.5"
                        >
                            <div
                                className={
                                    isToday(day)
                                        ? 'flex size-5 items-center justify-center rounded-full bg-secondary text-xs text-primary'
                                        : 'text-xs text-primary/70'
                                }
                            >
                                {format(day, 'd')}
                            </div>
                            <div className="mt-1 space-y-1">
                                {dayEvents.map((event) => (
                                    <CalendarEntry
                                        key={`${event.uid}-${event.start}`}
                                        event={event}
                                    />
                                ))}
                            </div>
                        </div>
                    );
                })}
            </div>
        </section>
    );
};

const CalendarEntry: FC<{ event: CalendarEvent }> = ({ event }) => {
    const className =
        'block truncate rounded bg-secondary px-1 py-0.5 text-xs text-primary hover:bg-tertiary';

    if (event.pm_number) {
        return (
            <Link
                to="/pm/$issueId"
                params={{ issueId: event.pm_number.toString() }}
                className={className}
            >
                {event.summary}
            </Link>
        );
    }

    const meetingUrl = event.meetings[0]?.link;

    return meetingUrl ? (
        <a href={meetingUrl} target="_blank" rel="noreferrer" className={className}>
            {event.summary}
        </a>
    ) : (
        <div className={className}>{event.summary}</div>
    );
};
