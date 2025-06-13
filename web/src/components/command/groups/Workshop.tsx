import { useNavigate } from '@tanstack/react-router';
import { LuWandSparkles } from 'react-icons/lu';

import { useWorkshopSendMessage } from '@/api';

import { CommandGroup, CommandItem, CommandSeparator } from '../Command';
import { useCommand } from '../CommandMenu';

export const WorkshopIdea = () => {
    const { search, onOpenChange } = useCommand();
    const navigate = useNavigate();
    const { mutate } = useWorkshopSendMessage('new');

    if (!search) return null;

    return (
        <>
            <CommandGroup heading="Workshop">
                <CommandItem
                    value={`Workshop idea: ${search}`}
                    keywords={['workshop', 'idea', 'chat', 'ai']}
                    onSelect={() => {
                        mutate(
                            {
                                message: search,
                            },
                            {
                                onSuccess: (data) => {
                                    navigate({
                                        to: '/chat/$chatId',
                                        params: { chatId: data.chat_id },
                                        search: { q: search },
                                    });
                                    onOpenChange(false);
                                },
                            }
                        );
                    }}
                    className="flex flex-col items-start gap-2 px-3 py-1.5 data-[selected=true]:bg-secondary data-[selected=true]:text-primary"
                >
                    <div className="flex items-center gap-2 mb-1">
                        <LuWandSparkles className="size-5" />
                        {search}
                    </div>
                </CommandItem>
            </CommandGroup>
            <CommandSeparator />
        </>
    );
};
