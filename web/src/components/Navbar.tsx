import { Link, useMatches } from '@tanstack/react-router';
import classNames from 'classnames';
import { FC, useEffect, useState } from 'react';
import { SiEthereum } from 'react-icons/si';

import { SearchBar } from './search';

export const Navbar: FC = () => {
    const data = useMatches();

    const title = findMapReverse(data, (m) => {
        if ('title' in m.context) {
            if (m.context.title) return m.context.title as string;
        }
    });
    const route = data[data.length - 1].routeId;

    document.title = title ?? 'Ethereum Forum';

    return (
        <>
            <div className="w-full bg-secondary fixed top-0 grid grid-cols-[1fr_auto_1fr] max-h-16 z-10">
                <div className="flex items-stretch gap-2 px-3 h-full">
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
                    <div>
                        <SearchBar />
                    </div>
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
                <div className="items-center h-full gap-2 flex-1 justify-end px-2 text-sm hidden 2xl:flex">
                    {/* <UserProfile /> */}
                    {/* Last refreshed 2 min ago */}
                </div>
            </div>
            <div className="h-8 w-full" />
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
