import * as Dialog from '@radix-ui/react-dialog';
import { Link, useRouterState } from '@tanstack/react-router';
import classNames from 'classnames';
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

const SidebarContent: FC<{
    pathname: string;
    isAuthenticated: boolean;
    onNavigate?: () => void;
}> = ({ pathname, isAuthenticated, onNavigate }) => {
    return (
        <div className="space-y-2 bg-secondary h-screen max-h-screen min-h-screen">
            <div className="flex flex-col justify-between h-screen">
                <nav className="w-full space-y-6 p-2 h-8">
                    <div>
                        <div className="flex items-stretch gap-2 h-full px-2">
                            <Link
                                to="/"
                                onClick={onNavigate}
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
                                    onClick={onNavigate}
                                    className="flex justify-between items-center hover:bg-tertiary rounded-md px-2 py-1 relative"
                                >
                                    <div className="flex items-center gap-2">
                                        <div>{item.icon}</div>
                                        <span>{item.title}</span>
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
                                onClick={onNavigate}
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
                                onClick={onNavigate}
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
                </div>
            </div>
        </div>
    );
};

export const Sidebar: FC = () => {
    const { pathname } = useRouterState({ select: (s) => s.location });
    const { isAuthenticated } = useAuth();
    const { isSidebarOpen, closeSidebar } = useApp();

    return (
        <>
            {/* Desktop sidebar */}
            <div
                className={classNames(
                    'left-bar hidden md:block bg-secondary border-r border-r-secondary h-screen max-h-screen min-h-screen sticky top-0 transition-all duration-300',
                    isSidebarOpen
                        ? 'w-[240px] min-w-[180px] max-w-xs'
                        : 'w-0 overflow-hidden !border-none'
                )}
            >
                {isSidebarOpen && (
                    <SidebarContent pathname={pathname} isAuthenticated={isAuthenticated} />
                )}
            </div>

            {/* Mobile sidebar overlay */}
            <Dialog.Root
                open={isSidebarOpen}
                onOpenChange={(v) => {
                    if (!v) closeSidebar();
                }}
            >
                <Dialog.Portal>
                    <Dialog.Overlay className="fixed inset-0 bg-black/50 z-40 md:hidden data-[state=open]:animate-overlayShow" />
                    <Dialog.Content className="fixed inset-y-0 left-0 z-50 bg-secondary w-64 md:hidden outline-none data-[state=open]:animate-contentShow">
                        <SidebarContent
                            pathname={pathname}
                            isAuthenticated={isAuthenticated}
                            onNavigate={closeSidebar}
                        />
                    </Dialog.Content>
                </Dialog.Portal>
            </Dialog.Root>
        </>
    );
};
