import { useEffect, useState } from 'react';
import { FiMonitor, FiMoon, FiSun } from 'react-icons/fi';

import { updateTheme } from '@/components/preferences/ThemeSwitcher';

import { CommandGroup, CommandItem } from '../Command';
import { useCommand } from '../CommandMenu';

const themeOptions = {
    light: { label: 'Light Mode', icon: <FiSun className="size-5" /> },
    dark: { label: 'Dark Mode', icon: <FiMoon className="size-5" /> },
    system: { label: 'System', icon: <FiMonitor className="size-5" /> },
};

export const ThemeSwitcherCommand = () => {
    const { onOpenChange } = useCommand();
    const [currentTheme, setCurrentTheme] = useState<string>(
        () => localStorage.getItem('color-theme') || 'system'
    );

    useEffect(() => {
        const stored = localStorage.getItem('color-theme') || 'system';

        setCurrentTheme(stored);
    }, []);

    const handleSwitch = (theme: string) => {
        localStorage.setItem('color-theme', theme);
        setCurrentTheme(theme);
        updateTheme();
        onOpenChange(false);
    };

    return (
        <CommandGroup>
            <div className="flex items-center gap-2 my-2 text-base font-semibold">
                {themeOptions[currentTheme as keyof typeof themeOptions]?.icon}
                Theme
            </div>

            {Object.entries(themeOptions).map(([key, opt]) =>
                key === currentTheme ? null : (
                    <CommandItem
                        key={key}
                        onSelect={() => handleSwitch(key)}
                        value={key}
                        keywords={[
                            'theme',
                            'switch',
                            'mode',
                            'light mode',
                            'dark mode',
                            'system',
                            ' ',
                        ]}
                    >
                        {opt.icon}
                        <span>Switch to {opt.label}</span>
                    </CommandItem>
                )
            )}
        </CommandGroup>
    );
};
