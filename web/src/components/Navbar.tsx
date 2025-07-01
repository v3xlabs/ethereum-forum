import { useMatches } from '@tanstack/react-router';
import classNames from 'classnames';
import { FC, useEffect, useState } from 'react';
import { FiLogOut, FiSearch, FiSidebar, FiUser } from 'react-icons/fi';

import { useAuth, useLogout } from '../api/auth';
import { useApp } from '../hooks/context';
import { LoginButton } from './LoginButton';
import { CommandMenu } from './command/CommandMenu';

export const Navbar: FC = () => {
    const data = useMatches();
    const { isAuthenticated, user, isLoading } = useAuth();
    const logoutMutation = useLogout();
    const { isSidebarOpen, toggleSidebar } = useApp();

    const title = findMapReverse(data, (m) => {
        if ('title' in m.context) {
            if (m.context.title) return m.context.title as string;
        }
    });

    document.title = title ?? 'Ethereum Forum';

    const handleLogout = () => {
        logoutMutation.mutate();
    };

    return (
        <>
            <div className="w-full bg-primary sticky top-0 grid grid-cols-[1fr_auto_1fr] h-10 z-10 px-2 border-b border-b-secondary right-0 left-0">
                <div className="flex items-center justify-start w-fit">
                    <button
                        className="button aspect-square border-none"
                        onClick={toggleSidebar}
                        aria-label={isSidebarOpen ? 'Hide sidebar' : 'Show sidebar'}
                    >
                        {isSidebarOpen ? <FiSidebar /> : <FiSidebar />}
                    </button>
                </div>
                <div className={classNames('w-full h-full flex items-center')}>
                    <div className="px-2 truncate only-after-scroll font-bold transition-all duration-300">
                        {title || ''}
                    </div>
                </div>
                <div className="items-center h-full gap-2 flex-1 justify-end px-2 text-sm hidden md:flex">
                    <SearchButton />
                    {isLoading ? (
                        <div className="flex items-center gap-2 px-3 py-1 text-sm text-primary">
                            <FiUser size={16} />
                            Loading...
                        </div>
                    ) : isAuthenticated && user ? (
                        <div className="flex items-center gap-2">
                            <div className="flex items-center gap-2 px-2 py-1 text-sm">
                                <FiUser size={16} />
                                <span>{user.display_name || user.username || user.email}</span>
                            </div>
                            <button
                                onClick={handleLogout}
                                disabled={logoutMutation.isPending}
                                className="flex items-center gap-1 px-2 py-1 rounded-md text-sm hover:bg-secondary transition-colors disabled:opacity-50"
                                title="Sign out"
                            >
                                <FiLogOut size={14} />
                                <span className="hidden lg:inline">
                                    {logoutMutation.isPending ? 'Signing out...' : 'Sign out'}
                                </span>
                            </button>
                        </div>
                    ) : (
                        <LoginButton />
                    )}
                    {/* Last refreshed 2 min ago */}
                </div>
            </div>
            <ScrollListener />
        </>
    );
};

function findMapReverse<T, U>(data: T[], fn: (t: T) => U | undefined): U | undefined {
    for (let i = data.length - 1; i >= 0; i--) {
        const t = data[i];
        const u = fn(t);

        if (u) return u;
    }
}

const ScrollListener = () => {
    const [scrolled, setScrolled] = useState(false);

    useEffect(() => {
        const handleScroll = () => {
            const h1Element = document.querySelector('h1');
            const y = h1Element?.getBoundingClientRect().top || 0;

            if (h1Element && y < 42) {
                setScrolled(true);
            } else {
                setScrolled(false);
            }
        };

        window.addEventListener('scroll', handleScroll);

        return () => {
            window.removeEventListener('scroll', handleScroll);
            document.documentElement.classList.remove('scrolled');
        };
    }, []);

    useEffect(() => {
        if (scrolled) {
            document.documentElement.classList.add('scrolled');
        } else {
            document.documentElement.classList.remove('scrolled');
        }
    }, [scrolled]);

    return <></>;
};

export const SearchButton = () => {
    const [isOpen, setIsOpen] = useState(false);

    return (
        <>
            <button className="button aspect-square border-none" onClick={() => setIsOpen(true)}>
                <FiSearch />
            </button>

            <CommandMenu triggerOpen={isOpen} onClose={() => setIsOpen(false)} />
        </>
    );
};
