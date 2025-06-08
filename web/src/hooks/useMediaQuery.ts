import { useEffect, useState } from 'react';

export const useMediaQuery = (query: string) => {
    const [matches, setMatches] = useState(
        typeof window !== 'undefined' ? window.matchMedia(query).matches : false
    );

    useEffect(() => {
        if (typeof window === 'undefined') return;

        const media = window.matchMedia(query);
        const handler = () => setMatches(media.matches);

        handler();
        media.addEventListener('change', handler);

        return () => media.removeEventListener('change', handler);
    }, [query]);

    return matches;
};
