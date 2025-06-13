import * as React from 'react';

import { CommandGroup, CommandItem } from '../Command';
import { useCommand } from '../CommandMenu';

// Navigation items from Sidebar
const navItems = [
    { title: 'Index', href: '/', short: 'Everything' },
    { title: 'Roadmap', href: '/r', short: 'Hardforks' },
    { title: 'Standards', href: '/s', short: 'EIPs & ERCs' },
    { title: 'Protocol Agenda', href: '/c', short: 'Calendar' },
];

export const Navigation: React.FC = () => {
    const { handleSelect } = useCommand();

    return (
        <CommandGroup heading="Navigation">
            {navItems.map((item) => (
                <CommandItem
                    key={item.href}
                    onSelect={() => handleSelect(item)}
                    className="flex justify-between data-[selected=true]:bg-secondary data-[selected=true]:text-primary"
                >
                    {item.title}
                    {item.short && (
                        <span className="ml-2 text-xs text-secondary">{item.short}</span>
                    )}
                </CommandItem>
            ))}
        </CommandGroup>
    );
};
