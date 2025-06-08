import { ReactNode } from 'react';

import { useMediaQuery } from '@/hooks/useMediaQuery';

import { RightbarHamburger } from './RightbarHamburger';

export const ResponsiveRightbar = ({ children }: { children: ReactNode }) => {
    const isDesktop = useMediaQuery('(min-width: 768px)');

    return isDesktop ? (
        <div className="right-bar p-4">{children}</div>
    ) : (
        <RightbarHamburger>{children}</RightbarHamburger>
    );
};
