import {
    addDays,
    eachDayOfInterval,
    format,
    isSameDay,
    isToday,
    startOfDay,
    startOfWeek,
} from 'date-fns';
import { FC } from 'react';

import { CalendarEvent } from '@/api/events';

const weekdayLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

export const CalendarOverview: FC<{ data: CalendarEvent[] }> = ({ data }) => {
    const calendarStart = startOfWeek(startOfDay(new Date()), { weekStartsOn: 1 });
    const calendarDays = eachDayOfInterval({
        start: calendarStart,
        end: addDays(calendarStart, 34),
    });

    return (
        <section className="card space-y-3 overflow-x-auto" aria-labelledby="agenda-calendar-title">
            <div className="flex items-baseline justify-between gap-3">
                <div>
                    <h2 id="agenda-calendar-title" className="font-bold">
                        Calendar
                    </h2>
                    <p className="text-sm text-primary/70">The next five weeks at a glance</p>
                </div>
                <span className="shrink-0 text-sm text-primary/70">All times are local</span>
            </div>
            <div className="grid min-w-[560px] grid-cols-7 border-l border-t border-primary/50">
                {weekdayLabels.map((weekday) => (
                    <div
                        key={weekday}
                        className="border-b border-r border-primary/50 px-2 py-1 text-center text-xs text-primary/70"
                    >
                        {weekday}
                    </div>
                ))}
                {calendarDays.map((day) => {
                    const dayEvents = data.filter(
                        (event) => event.start && isSameDay(event.start, day)
                    );
                    const dayId = `agenda-day-${format(day, 'yyyy-MM-dd')}`;

                    return (
                        <a
                            key={dayId}
                            href={`#${dayId}`}
                            className="min-h-24 border-b border-r border-primary/50 p-1.5 hover:bg-secondary/50"
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
                                {dayEvents.slice(0, 2).map((event) => (
                                    <div
                                        key={`${event.uid}-${event.start}`}
                                        className="truncate rounded bg-secondary px-1 py-0.5 text-xs text-primary"
                                        title={event.summary}
                                    >
                                        {event.summary}
                                    </div>
                                ))}
                                {dayEvents.length > 2 && (
                                    <div className="text-xs text-primary/70">
                                        +{dayEvents.length - 2} more
                                    </div>
                                )}
                            </div>
                        </a>
                    );
                })}
            </div>
        </section>
    );
};
