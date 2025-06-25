import { Link, useRouterState } from '@tanstack/react-router';
import { FC } from 'react';
import { FiLock } from 'react-icons/fi';
import { GrWorkshop } from 'react-icons/gr';
import { LuBook, LuCalendar, LuHouse, LuNewspaper, LuWrench } from 'react-icons/lu';
import { SiEthereum, SiOpenai } from 'react-icons/si';

import { useAuth } from '../api/auth';
import { useApp } from '../hooks/context';
import { ProseWidthSwitcher } from './preferences/ProseWidthSwitcher';
import { ThemeSwitcher } from './preferences/ThemeSwitcher';
import { WorkshopChatsNav } from './workshop/WorkshopChatsNav';

export const NAV_ITEMS = [
    {
        title: 'Home',
        href: '/',
        short: 'Everything',
        icon: <LuHouse />,
    },
    {
        title: 'Protocol Agenda',
        href: '/c',
        short: 'Calendar',
        icon: <LuCalendar />,
    },
    {
        title: 'Standards',
        href: '/s',
        short: 'EIPs & ERCs',
        icon: <LuBook />,
    },
    {
        title: 'Roadmap',
        href: '/r',
        short: 'Hardforks',
        icon: <LuNewspaper />,
    },
    {
        title: 'Workshop',
        href: '/chat/new',
        requiresAuth: true,
        icon: <LuWrench />,
    },
];

export const Sidebar: FC = () => {
    const { pathname } = useRouterState({ select: (s) => s.location });
    const { isAuthenticated } = useAuth();
    const { isSidebarOpen } = useApp();

    if (!isSidebarOpen) {
        // Keep the sidebar width for layout, but hide content
        return (
            <div
                className="left-bar w-[240px] min-w-[180px] max-w-xs bg-secondary border-r border-r-secondary h-screen max-h-screen min-h-screen sticky top-0 transition-all duration-300"
                style={{
                    width: 0,
                    minWidth: 0,
                    maxWidth: 0,
                    overflow: 'hidden',
                    padding: 0,
                    border: 'none',
                }}
            />
        );
    }

    return (
        <div className="left-bar space-y-2 bg-secondary border-r border-r-secondary h-screen max-h-screen min-h-screen sticky top-0">
            <div className="flex flex-col justify-between h-screen">
                <nav className="w-full space-y-6 p-2 h-8">
                    <div>
                        <div className="flex items-stretch gap-2 h-full px-2">
                            <Link
                                to="/"
                                className="text-primary font-bold text-base hover:underline py-1 flex items-center gap-1"
                            >
                                <SiEthereum />
                                <span className="hidden lg:block">
                                    <span>ethereum</span>
                                    <span className="text-secondary">.</span>
                                    <span>forum</span>
                                </span>
                            </Link>
                        </div>
                    </div>
                    <ul className="overflow-hidden space-y-1">
                        {NAV_ITEMS.map((item) => (
                            <li key={item.href} className="group">
                                <Link
                                    to={item.href}
                                    className="flex justify-between items-center hover:bg-tertiary rounded-md px-2 py-1.5 relative"
                                >
                                    <div className="flex items-center gap-2">
                                        <div className="">{item.icon}</div>
                                        <span className="">{item.title}</span>
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
                <div className="py-4 px-4 space-y-0.5">
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
                                className="text-sm button aspect-square size-8 flex items-center justify-center"
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
