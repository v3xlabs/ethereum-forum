import classNames from 'classnames';
import { FC, useState } from 'react';
import { LuCalendar } from 'react-icons/lu';
import { match } from 'ts-pattern';

import { useEvents, useEventsRecent, useEventsUpcoming } from '@/api/events';

import { CalendarOverview } from './CalendarOverview';
import { Meetings } from './Meetings';
import { AgendaVideos } from './Videos';

export const AgendaContent: FC = () => {
    const [tab, setTab] = useState<'upcoming' | 'recent' | 'videos'>('upcoming');
    const { data: events } = useEvents();
    const { data: upcoming } = useEventsUpcoming();
    const { data: recent } = useEventsRecent();

    return (
        <>
            <div className="card">
                <div className="flex flex-col gap-3 sm:flex-row sm:justify-between">
                    <div className="self-start">
                        <h2 className="text-lg font-bold">Protocol Agenda</h2>
                        <p>
                            the protocol agenda is a shared calendar that includes all meetings and
                            live streams around the protocol.
                        </p>
                        <p>
                            See also{' '}
                            <a
                                href="https://github.com/ethereum/pm/issues"
                                target="_blank"
                                rel="noreferrer"
                                className="link"
                            >
                                github.com/ethereum/pm/issues
                            </a>
                        </p>
                    </div>
                    <div>
                        <a
                            href="https://calendar.google.com/calendar/u/0?cid=Y191cGFvZm9uZzhtZ3JtcmtlZ243aWM3aGs1c0Bncm91cC5jYWxlbmRhci5nb29nbGUuY29t"
                            target="_blank"
                            className="button px-1 flex items-center gap-1"
                            rel="noreferrer"
                        >
                            <LuCalendar />
                            <span>Add to calendar</span>
                        </a>
                    </div>
                </div>
            </div>
            <div className="flex gap-1">
                <button
                    className={classNames('tab button', tab === 'upcoming' && 'active')}
                    onClick={() => setTab('upcoming')}
                >
                    Upcoming
                </button>
                <button
                    className={classNames('tab button', tab === 'recent' && 'active')}
                    onClick={() => setTab('recent')}
                >
                    Recent
                </button>
                <button
                    className={classNames('tab button', tab === 'videos' && 'active')}
                    onClick={() => setTab('videos')}
                >
                    Videos
                </button>
            </div>
            {match(tab)
                .with('upcoming', () => (
                    <div className="space-y-4">
                        <CalendarOverview data={[...(recent ?? []), ...(events ?? [])]} />
                        <Meetings data={upcoming ?? []} key="upcoming" />
                    </div>
                ))
                .with('recent', () => <Meetings data={recent ?? []} key="recent" />)
                .with('videos', () => <AgendaVideos />)
                .exhaustive()}
        </>
    );
};
