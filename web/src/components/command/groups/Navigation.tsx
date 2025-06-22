import * as React from 'react';
import { FiNavigation } from 'react-icons/fi';

import { CommandGroup, CommandItem } from '../Command';
import { useCommand } from '../CommandMenu';

// Navigation items from Sidebar
const navItems = [
    { title: 'Index', href: '/', short: 'Everything' },
    { title: 'Roadmap', href: '/r', short: 'Hardforks' },
    { title: 'Standards', href: '/s', short: 'EIPs & ERCs' },
    { title: 'Protocol Agenda', href: '/c', short: 'Calendar' },
    { title: 'Workshop', href: '/chat/new' },
];

export const Navigation: React.FC = () => {
    const { handleSelect } = useCommand();

    return (
        <CommandGroup>
            <div className="flex items-center gap-2 my-2 text-base font-semibold">
                <FiNavigation className="size-5" />
                Navigation
            </div>
            {navItems.map((item) => (
                <CommandItem key={item.href} onSelect={() => handleSelect(item)}>
                    <span className="flex items-center w-full justify-between">
                        {item.title}
                        {item.short && (
                            <span className="ml-2 text-xs text-secondary">{item.short}</span>
                        )}
                    </span>
                </CommandItem>
            ))}
        </CommandGroup>
    );
};
