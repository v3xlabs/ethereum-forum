import { useNavigate } from '@tanstack/react-router';
import { GrWorkshop } from 'react-icons/gr';

import { CommandGroup, CommandItem } from '../Command';
import { useCommand } from '../CommandMenu';

export const WorkshopIdea = () => {
    const { search, onOpenChange } = useCommand();
    const navigate = useNavigate();

    if (!search || search.length < 2) return null;

    return (
        <CommandGroup>
            <div className="flex items-center gap-2 my-2 text-base font-semibold">
                <GrWorkshop className="size-5" />
                Workshop
            </div>
            <CommandItem
                value={`Workshop idea: ${search} `}
                keywords={['workshop idea', 'workshop', 'idea', 'ai', 'chat', ' ']}
                onSelect={() => {
                    navigate({
                        to: '/chat/$chatId',
                        params: { chatId: 'new' },
                        search: { q: search },
                    });
                    onOpenChange(false);
                }}
            >
                <div className="flex flex-col items-start gap-1">
                    <div className="text-base text-primary/80 pl-7 break-words w-full text-left">
                        {search}
                    </div>
                </div>
            </CommandItem>
        </CommandGroup>
    );
};
