import { format, isToday, isTomorrow, parseISO } from 'date-fns';
import { FC } from 'react';

import { CalendarEvent } from '@/api/events';

import { MeetingCard } from './Meetings';

const DayHeader: FC<{ date: string }> = ({ date }) => {
    let prefix = format(date, 'MMM d');

    if (isToday(date)) prefix = 'Today';

    if (isTomorrow(date)) prefix = 'Tomorrow';

    return (
        <h2 className="text-base text-primary py-4">
            <span className="font-semibold">{prefix}</span>
            <span> -</span>
            <span className="font-normal text-primary/70"> {format(date, 'EEEE')}</span>
        </h2>
    );
};

export const CalendarDays: FC<{ data: CalendarEvent[] }> = ({ data }) => {
    const groupedDays = Object.values(
        data.reduce(
            (day, event) => {
                if (!event.start) return day;

                const date = parseISO(event.start).toDateString();

                if (!day[date]) {
                    day[date] = { date, events: [] };
                }

                day[date].events.push(event);

                return day;
            },
            {} as Record<string, { date: string; events: CalendarEvent[] }>
        )
    );

    return (
        <div>
            {groupedDays.map(({ date, events }) => (
                <div key={date} className="flex gap-5">
                    {/* timeline */}
                    <div className="relative flex flex-col items-center">
                        <div className="absolute top-6 h-2 w-2 rounded-full bg-grey" />
                        <div className="absolute top-6 h-full border-r border-dashed border-primary" />
                    </div>

                    <div className="flex-1">
                        <DayHeader date={date} />

                        {/* meeting cards */}
                        <div className="space-y-3 pb-1">
                            {events.map((event) => (
                                <MeetingCard key={`${event.uid}-${event.start}`} event={event} />
                            ))}
                        </div>
                    </div>
                </div>
            ))}
        </div>
    );
};
