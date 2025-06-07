import { Link, useRouterState } from '@tanstack/react-router';
import classNames from 'classnames';
import { FC, ReactNode, useEffect } from 'react';
import { FiLock, FiX } from 'react-icons/fi';
import { GrWorkshop } from 'react-icons/gr';
import { SiOpenai } from 'react-icons/si';

import { useAuth } from '../api/auth';
import { ProseWidthSwitcher } from './preferences/ProseWidthSwitcher';
import { ThemeSwitcher } from './preferences/ThemeSwitcher';
import { WorkshopChatsNav } from './workshop/WorkshopChatsNav';

interface MobileMenuProps {
    isOpen: boolean;
    onClose: () => void;
    side: 'left' | 'right';
    rightContent?: ReactNode;
}

export const MobileMenu: FC<MobileMenuProps> = ({ isOpen, onClose, side, rightContent }) => {
    const { pathname } = useRouterState({ select: (s) => s.location });
    const { isAuthenticated } = useAuth();

    // Close menu on route change
    useEffect(() => {
        if (isOpen) {
            onClose();
        }
    }, [pathname]); // Remove onClose from deps to prevent unnecessary re-runs

    // Close menu on escape key
    useEffect(() => {
        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                onClose();
            }
        };

        if (isOpen) {
            document.addEventListener('keydown', handleEscape);
            // Prevent body scroll when menu is open
            document.body.style.overflow = 'hidden';
        } else {
            document.body.style.overflow = '';
        }

        return () => {
            document.removeEventListener('keydown', handleEscape);
            document.body.style.overflow = '';
        };
    }, [isOpen, onClose]);

    if (!isOpen) return null;

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-primary/80 backdrop-blur-sm z-40 md:hidden"
                onClick={onClose}
            />

            {/* Menu Panel */}
            <div
                onClick={(e) => e.stopPropagation()}
                className={classNames(
                    'fixed top-0 bottom-0 w-80 max-w-[85vw] bg-primary border-r border-primary z-50 md:hidden',
                    'transform transition-transform duration-300 ease-in-out',
                    side === 'left' ? 'left-0' : 'right-0',
                    isOpen
                        ? 'translate-x-0'
                        : side === 'left'
                          ? '-translate-x-full'
                          : 'translate-x-full'
                )}
            >
                <div className="flex flex-col h-full">
                    {/* Header */}
                    <div className="flex items-center justify-between p-4 border-b border-primary">
                        <h2 className="font-bold text-lg">
                            {side === 'left' ? 'Navigation' : 'Menu'}
                        </h2>
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                onClose();
                            }}
                            className="p-1 hover:bg-secondary rounded-md transition-colors"
                        >
                            <FiX size={20} />
                        </button>
                    </div>

                    {/* Content */}
                    <div className="flex-1 overflow-y-auto">
                        {side === 'left' ? (
                            <LeftSidebarContent 
                                pathname={pathname}
                                isAuthenticated={isAuthenticated}
                            />
                        ) : (
                            <div className="p-4">
                                {rightContent || <div className="text-secondary">No additional content</div>}
                            </div>
                        )}
                    </div>

                    {/* Footer for left sidebar */}
                    {side === 'left' && (
                        <div className="border-t border-primary p-4 space-y-3">
                            <div className="flex items-center justify-between gap-1">
                                <span className="text-sm">Explore</span>
                                <div className="flex items-center gap-1">
                                    <Link
                                        to="/chat/$chatId"
                                        params={{ chatId: 'new' }}
                                        className="text-sm button flex items-center justify-center gap-1"
                                    >
                                        <GrWorkshop />
                                        Workshop
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
                        </div>
                    )}
                </div>
            </div>
        </>
    );
};

const LeftSidebarContent: FC<{ pathname: string; isAuthenticated: boolean }> = ({
    pathname,
    isAuthenticated,
}) => {
    return (
        <nav className="w-full space-y-1.5 p-4">
            <ul className="space-y-1">
                {[
                    {
                        title: 'Index',
                        href: '/',
                        short: 'Everything',
                    },
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
                    <li key={item.href}>
                        <Link
                            to={item.href}
                            className="flex justify-between items-center hover:bg-secondary px-3 py-2 rounded-md transition-colors"
                        >
                            <div className="flex items-center gap-2">
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
                                <div className="mt-2 ml-4">
                                    <WorkshopChatsNav />
                                </div>
                            )}
                    </li>
                ))}
            </ul>
        </nav>
    );
}; 