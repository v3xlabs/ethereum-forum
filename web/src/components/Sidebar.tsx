import { Link, useRouterState } from '@tanstack/react-router';
import classNames from 'classnames';
import { FiLock } from 'react-icons/fi';
import { GrWorkshop } from 'react-icons/gr';
import { SiOpenai } from 'react-icons/si';

import { useAuth } from '../api/auth';
import { ProseWidthSwitcher } from './preferences/ProseWidthSwitcher';
import { ThemeSwitcher } from './preferences/ThemeSwitcher';
import { WorkshopChatsNav } from './workshop/WorkshopChatsNav';

export const Sidebar = ({ className = '' }: { className?: string } = {}) => {
    const { pathname } = useRouterState({ select: (s) => s.location });
    const { isAuthenticated } = useAuth();

    return (
        <div className={classNames('left-bar space-y-2', className)}>
            <div className="flex flex-col justify-between h-full">
                <nav className="w-full space-y-1.5 p-4">
                    <div className="px-1.5">
                        <p className="font-bold w-full border-b border-b-primary pb-1">
                            Navigation
                        </p>
                    </div>
                    <ul className="overflow-hidden">
                        {[
                            {
                                title: 'Index',
                                href: '/',
                                short: 'Everything',
                            },
                            // {
                            //     title: 'Improvement Proposals',
                            //     short: 'EIPs',
                            //     href: '/c/eips',
                            // },
                            // {
                            //     title: 'Request for Comment',
                            //     short: 'ERCs',
                            //     href: '/c/ercs',
                            // },
                            // {
                            //     title: 'Working Groups',
                            //     href: '/c/wgs',
                            // },
                            {
                                title: 'Roadmap',
                                href: '/r',
                                short: 'Hardforks',
                            },
                            {
                                title: 'Standards',
                                href: '/s',
                                short: 'EIPs & ERCs',
                            },
                            {
                                title: 'Protocol Agenda',
                                href: '/c',
                                short: 'Calendar',
                            },
                            {
                                title: 'Workshop',
                                href: '/chat/new',
                                requiresAuth: true,
                            },
                        ].map((item) => (
                            <li key={item.href} className="group">
                                <Link
                                    to={item.href}
                                    className="flex justify-between items-center hover:bg-secondary px-1.5 py-0.5 relative"
                                >
                                    <div className="flex items-center gap-2">
                                        <div className="absolute top-0 left-2 w-2 h-full border-l-2 border-primary group-last:h-1/2"></div>
                                        <div className="absolute top-0 left-2 w-2 h-full border-primary border-b-2 -translate-y-1/2"></div>
                                        <span className="pl-4">{item.title}</span>
                                        {item.requiresAuth && !isAuthenticated && (
                                            <FiLock size={12} className="text-primary opacity-60" />
                                        )}
                                    </div>
                                    {item.short && (
                                        <span className="text-sm text-secondary text-right">
                                            {item.short}
                                        </span>
                                    )}
                                </Link>
                                {item.href === '/chat/new' &&
                                    pathname.startsWith('/chat') &&
                                    isAuthenticated && (
                                        <div className="pl-4">
                                            <WorkshopChatsNav />
                                        </div>
                                    )}
                            </li>
                        ))}
                    </ul>
                </nav>
                <div className="py-4 px-6 space-y-0.5">
                    <div className="flex items-center justify-between gap-1">
                        <span className="text-sm">Explore</span>
                        <div className="flex items-center gap-1">
                            <Link
                                to="/chat/$chatId"
                                params={{ chatId: 'new' }}
                                className="text-sm button flex items-center justify-center gap-1"
                            >
                                <GrWorkshop />
                                Open Workshop
                                {!isAuthenticated && <FiLock size={12} className="opacity-60" />}
                            </Link>
                            <a
                                href="https://chatgpt.com/g/g-68104906afb88191ae3f52c2aff36737-ethereum-forum-assistant"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-sm button aspect-square size-7 flex items-center justify-center"
                            >
                                <SiOpenai />
                            </a>
                        </div>
                    </div>
                    <div className="flex items-center justify-between gap-1">
                        <span className="text-sm">Theme</span>
                        <ThemeSwitcher />
                    </div>
                    <div className="flex items-center justify-between gap-1">
                        <span className="text-sm">Text Width</span>
                        <ProseWidthSwitcher />
                    </div>
                    {/* <div className="flex items-center justify-between gap-1 pr-1">
                        <span className="text-sm">Last refreshed</span>
                        <span className="text-sm">2 min ago</span>
                    </div> */}
                </div>
            </div>
        </div>
    );
};
