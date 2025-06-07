import { Link, useMatches } from '@tanstack/react-router';
import classNames from 'classnames';
import { FC, ReactNode, useEffect, useState } from 'react';
import { FiLogOut, FiMenu, FiUser } from 'react-icons/fi';
import { SiEthereum } from 'react-icons/si';

import { useAuth, useLogout } from '../api/auth';
import { LoginButton } from './LoginButton';
import { MobileMenu } from './MobileMenu';

interface NavbarProps {
    rightContent?: ReactNode;
}

export const Navbar: FC<NavbarProps> = ({ rightContent }) => {
    const data = useMatches();
    const { isAuthenticated, user, isLoading } = useAuth();
    const logoutMutation = useLogout();
    const [leftMenuOpen, setLeftMenuOpen] = useState(false);
    const [rightMenuOpen, setRightMenuOpen] = useState(false);

    const title = findMapReverse(data, (m) => {
        if ('title' in m.context) {
            if (m.context.title) return m.context.title as string;
        }
    });
    const route = data[data.length - 1].routeId;

    document.title = title ?? 'Ethereum Forum';

    const handleLogout = () => {
        logoutMutation.mutate();
    };

    return (
        <>
            <div className="w-full bg-secondary fixed top-0 grid grid-cols-[auto_1fr_auto] md:grid-cols-[1fr_auto_1fr] h-8 z-10">
                <div className="flex items-stretch gap-2 h-full px-3">
                    {/* Mobile hamburger menu button */}
                    <button
                        onClick={(e) => {
                            e.stopPropagation();
                            setLeftMenuOpen(true);
                        }}
                        className="md:hidden flex items-center justify-center p-1 hover:bg-primary rounded-md transition-colors"
                        aria-label="Open navigation menu"
                    >
                        <FiMenu size={16} />
                    </button>
                    
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
                <div
                    className={classNames(
                        'w-full h-full flex items-center',
                        route.startsWith('/t/') ? 'prose-width' : 'max-w-[1032px]'
                    )}
                >
                    <div className="px-2 truncate only-after-scroll font-bold transition-all duration-300">
                        {title}
                    </div>
                </div>
                <div className="items-center h-full gap-2 flex-1 justify-end px-2 text-sm flex">
                    {/* Desktop auth section */}
                    <div className="hidden md:flex items-center gap-2">
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
                    </div>

                    {/* Mobile right menu button - only show if there's right content */}
                    {rightContent && (
                        <button
                            onClick={(e) => {
                                e.stopPropagation();
                                setRightMenuOpen(true);
                            }}
                            className="md:hidden flex items-center justify-center p-1 hover:bg-primary rounded-md transition-colors"
                            aria-label="Open menu"
                        >
                            <FiMenu size={16} />
                        </button>
                    )}

                    {/* Mobile auth for when no right content */}
                    {!rightContent && (
                        <div className="md:hidden flex items-center gap-2">
                            {isAuthenticated && user ? (
                                <button
                                    onClick={handleLogout}
                                    disabled={logoutMutation.isPending}
                                    className="flex items-center gap-1 px-2 py-1 rounded-md text-sm hover:bg-secondary transition-colors disabled:opacity-50"
                                    title="Sign out"
                                >
                                    <FiLogOut size={14} />
                                </button>
                            ) : (
                                <LoginButton />
                            )}
                        </div>
                    )}
                </div>
            </div>
            <div className="h-8 w-full" />
            <ScrollListener />
            
            {/* Mobile Menus */}
            <MobileMenu
                isOpen={leftMenuOpen}
                onClose={() => setLeftMenuOpen(false)}
                side="left"
            />
            <MobileMenu
                isOpen={rightMenuOpen}
                onClose={() => setRightMenuOpen(false)}
                side="right"
                rightContent={rightContent}
            />
        </>
    );
};

// function findMap<T, U>(data: T[], fn: (t: T) => U | undefined): U | undefined {
//     for (const t of data) {
//         const u = fn(t);

//         if (u) return u;
//     }
// }

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
