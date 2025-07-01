import { useNavigate } from '@tanstack/react-router';
import {
    createContext,
    Dispatch,
    type FC,
    SetStateAction,
    useContext,
    useEffect,
    useState,
} from 'react';

import { CommandDialog, CommandInput, CommandList } from './Command';
import { Navigation } from './groups/Navigation';
// import { Suggested } from './groups/Suggested';
import { UpcomingCalendarEvent } from './groups/Upcoming';
import { WorkshopIdea } from './groups/Workshop';

interface CommandContextType {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onSelect?: (item: { title: string; href: string; short?: string }) => void;
    search: string;
    setSearch: Dispatch<SetStateAction<string>>;
    handleSelect: (item: { title: string; href: string; short?: string }) => void;
    handleClose: () => void;
}

const CommandContext = createContext<CommandContextType | undefined>(undefined);

export function useCommand() {
    const ctx = useContext(CommandContext);

    if (!ctx) throw new Error('useCommand must be used within a CommandMenu');

    return ctx;
}

export const CommandMenu: FC<{
    onSelect?: (item: { title: string; href: string; short?: string }) => void;
    triggerOpen?: boolean;
    onClose?: () => void;
}> = ({ onSelect, triggerOpen, onClose }) => {
    const navigate = useNavigate();
    const [open, setOpen] = useState(triggerOpen ?? false);

    const [search, setSearch] = useState('');

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
                e.preventDefault();
                setOpen((prev) => !prev);
            }
        };

        window.addEventListener('keydown', handleKeyDown);

        return () => window.removeEventListener('keydown', handleKeyDown);
    }, []);

    const handleSelect = (item: { title: string; href: string; short?: string }) => {
        if (onSelect) {
            onSelect(item);
        } else {
            navigate({ to: item.href });
        }

        handleClose();
    };

    const handleClose = () => {
        setOpen(false);
    };

    useEffect(() => {
        if (triggerOpen) {
            setOpen(true);
        }
    }, [triggerOpen]);

    useEffect(() => {
        if (triggerOpen && !open && onClose) {
            onClose();
        }
    }, [open]);

    const contextValue: CommandContextType = {
        open,
        onOpenChange: setOpen,
        onSelect,
        search,
        setSearch,
        handleSelect,
        handleClose,
    };

    return (
        <CommandContext.Provider value={contextValue}>
            <CommandDialog open={open} onOpenChange={setOpen}>
                <CommandInput
                    placeholder="Type a command or search..."
                    value={search}
                    onValueChange={setSearch}
                />
                <CommandList>
                    <UpcomingCalendarEvent />
                    {/* <Suggested /> */}
                    <Navigation />
                    <WorkshopIdea />
                </CommandList>
            </CommandDialog>
        </CommandContext.Provider>
    );
};
