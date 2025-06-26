import { format, isToday, isTomorrow, parseISO } from 'date-fns';
import { FC } from 'react';

import { CalendarEvent } from '@/api/events';

import { MeetingPreview } from './Meetings';

const DayHeader: FC<{ date: string }> = ({ date }) => {
    let prefix = format(date, 'MMM d');

    if (isToday(date)) prefix = 'today';

    if (isTomorrow(date)) prefix = 'tomorrow';

    return (
        <h2 className="text-lg font-semibold text-primary ">
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
        <div className="relative">
            {/* timeline */}
            <div className="absolute left-[0.23rem] top-5 bottom-0 w-px h-full border-l border-dashed border-primary" />

            {groupedDays.map(({ date, events }) => (
                <div key={date} className="flex gap-5">
                    <div className="relative">
                        {/* circle for timeline */}
                        <div className="absolute left-1 top-2.5 -translate-x-1/2 w-2 h-2 rounded-full bg-grey" />
                    </div>

                    <div className="w-full">
                        <DayHeader date={date} />

                        {events.map((event) => (
                            <MeetingPreview key={`${event.uid}-${event.start}`} event={event} />
                        ))}
                    </div>
                </div>
            ))}
        </div>
    );
};
