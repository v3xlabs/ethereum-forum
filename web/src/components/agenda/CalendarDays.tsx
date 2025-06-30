import { format, isToday, isTomorrow, parseISO } from 'date-fns';
import { FC } from 'react';

import { CalendarEvent } from '@/api/events';

import { MeetingCard } from './Meetings';

const DayHeader: FC<{ date: string }> = ({ date }) => {
    let prefix = format(date, 'MMM d');

    if (isToday(date)) prefix = 'today';

    if (isTomorrow(date)) prefix = 'tomorrow';

    return (
        <h2 className="text-md font-semibold text-primary py-6">
            <span className="font-semibold">{prefix}</span>
            <span className="font-normal"> {format(date, 'EEEE')}</span>
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
                        <div className="absolute top-8 h-2 w-2 rounded-full bg-grey" />
                        <div className="absolute top-8 h-full border-r border-dashed border-primary" />
                    </div>

                    <div className="min-w-full">
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
