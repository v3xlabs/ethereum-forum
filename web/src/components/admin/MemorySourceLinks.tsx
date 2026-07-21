import { Link } from '@tanstack/react-router';
import { LuExternalLink } from 'react-icons/lu';

import { MemoryLink } from '@/api/admin';

export const MemorySourceLinks = ({ sources }: { sources: MemoryLink[] }) => {
    if (sources.length === 0) return null;

    return (
        <ul className="mt-1.5 space-y-0.5 text-xs">
            {sources.map((source, index) => (
                <li key={index} className="flex items-baseline gap-1.5 min-w-0">
                    {source.url.startsWith('/') ? (
                        <Link
                            to={source.url}
                            className="text-secondary hover:underline truncate shrink-0 max-w-full"
                        >
                            {source.url}
                        </Link>
                    ) : (
                        <a
                            href={source.url}
                            target="_blank"
                            rel="noreferrer"
                            className="text-secondary hover:underline truncate shrink-0 max-w-full inline-flex items-center gap-1"
                        >
                            <span className="truncate">{source.url}</span>
                            <LuExternalLink className="w-3 h-3 shrink-0" />
                        </a>
                    )}
                    {source.reason && (
                        <span className="text-primary/40 truncate">— {source.reason}</span>
                    )}
                </li>
            ))}
        </ul>
    );
};
