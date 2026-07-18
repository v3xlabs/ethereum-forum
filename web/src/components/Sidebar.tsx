import { Link } from '@tanstack/react-router';
import { FC } from 'react';
import { LuBook, LuCalendar, LuExternalLink, LuHouse, LuNewspaper } from 'react-icons/lu';
import { SiEthereum } from 'react-icons/si';

import { useApp } from '../hooks/context';
import { ProseWidthSwitcher } from './preferences/ProseWidthSwitcher';
import { ThemeSwitcher } from './preferences/ThemeSwitcher';

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
];

export const Sidebar: FC = () => {
    const { isSidebarOpen, closeSidebar } = useApp();

    const closeSidebarOnMobile = () => {
        if (window.innerWidth < 768) {
            closeSidebar();
        }
    };

    return (
        <>
            {isSidebarOpen && (
                <button
                    type="button"
                    aria-label="Close sidebar"
                    className="fixed inset-0 z-20 bg-black/20 md:hidden"
                    onClick={closeSidebar}
                />
            )}
            <div
                className={`left-bar fixed inset-y-0 left-0 z-30 w-[min(85vw,340px)] space-y-2 bg-secondary border-r border-r-secondary transition-all duration-300 md:sticky md:top-0 md:z-auto md:h-screen md:min-h-screen md:w-full md:max-w-[340px] ${
                    isSidebarOpen
                        ? 'translate-x-0'
                        : '-translate-x-full md:w-0 md:min-w-0 md:max-w-0 md:overflow-hidden md:p-0 md:border-0'
                }`}
            >
                <div className="flex flex-col justify-between h-screen min-w-[240px]">
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
                                    {item.href === '/r' ? (
                                        <a
                                            href="https://forkcast.org"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            onClick={closeSidebarOnMobile}
                                            className="flex justify-between items-center hover:bg-tertiary rounded-md px-2 py-1.5 relative"
                                        >
                                            <div className="flex items-center gap-2">
                                                <div>{item.icon}</div>
                                                <span>{item.title}</span>
                                            </div>
                                            <span className="flex items-center gap-1 text-sm text-secondary text-right">
                                                {item.short}
                                                <LuExternalLink size={12} />
                                            </span>
                                        </a>
                                    ) : (
                                        <Link
                                            to={item.href}
                                            onClick={closeSidebarOnMobile}
                                            className="flex justify-between items-center hover:bg-tertiary rounded-md px-2 py-1.5 relative"
                                        >
                                            <div className="flex items-center gap-2">
                                                <div>{item.icon}</div>
                                                <span>{item.title}</span>
                                            </div>
                                            {item.short && (
                                                <span className="text-sm text-secondary text-right">
                                                    {item.short}
                                                </span>
                                            )}
                                        </Link>
                                    )}
                                </li>
                            ))}
                        </ul>
                    </nav>
                    <div className="py-4 px-4 space-y-0.5">
                        <div className="flex items-center justify-between gap-1">
                            <span className="text-sm">Theme</span>
                            <ThemeSwitcher />
                        </div>
                        <div className="flex items-center justify-between gap-1">
                            <span className="text-sm">Text Width</span>
                            <ProseWidthSwitcher />
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
};
