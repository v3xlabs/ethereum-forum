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
    const { isSidebarOpen } = useApp();

    return (
        <div
            className="left-bar space-y-2 bg-secondary border-r border-r-secondary h-screen max-h-screen min-h-screen sticky top-0 transition-all duration-300"
            style={{
                ...(!isSidebarOpen && {
                    width: 0,
                    minWidth: 0,
                    maxWidth: 0,
                    overflow: 'hidden',
                    padding: 0,
                    border: 'none',
                }),
            }}
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
    );
};
