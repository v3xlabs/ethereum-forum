import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import classNames from 'classnames';
import { FC, useState } from 'react';
import { FiChevronDown, FiMonitor, FiMoon, FiSun } from 'react-icons/fi';
import { TbPalette } from 'react-icons/tb';

export const updateTheme = () => {
    const theme = localStorage.getItem('color-theme') || 'system';

    document.documentElement.classList.remove('light', 'dark', 'solarized', 'system');
    document.documentElement.classList.add(theme);

    // Update theme-color meta tag for Apple's overscroll
    let metaThemeColor = document.querySelector('meta[name="theme-color"]') as HTMLMetaElement;

    if (!metaThemeColor) {
        metaThemeColor = document.createElement('meta');
        metaThemeColor.name = 'theme-color';
        document.head.appendChild(metaThemeColor);
    }

    // Define theme colors
    const themeColors = {
        light: 'rgb(255, 255, 255)', // --theme-bg-primary for light
        dark: 'rgb(0, 0, 0)', // --theme-bg-primary for dark
        solarized: 'rgb(253, 246, 227)', // --theme-bg-primary for solarized
        system: 'rgb(255, 255, 255)', // fallback, will be updated below
    };

    // Determine the actual theme to apply
    let actualTheme = theme;

    if (theme === 'system') {
        actualTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }

    // Set the theme color
    metaThemeColor.content = themeColors[actualTheme as keyof typeof themeColors];
};

const themeOptions = [
    { id: 'light', label: 'Light', icon: <FiSun size={16} /> },
    { id: 'dark', label: 'Dark', icon: <FiMoon size={16} /> },
    { id: 'solarized', label: 'Solarized', icon: <TbPalette size={16} /> },
    { id: 'system', label: 'System', icon: <FiMonitor size={16} /> },
];

export const ThemeSwitcher: FC = () => {
    const theme = localStorage.getItem('color-theme') || 'system';
    const [currentTheme, setCurrentTheme] = useState(theme);

    const setTheme = (themeId: string) => {
        localStorage.setItem('color-theme', themeId);
        setCurrentTheme(themeId);
        updateTheme();
    };

    const currentOption =
        themeOptions.find((option) => option.id === currentTheme) || themeOptions[3];

    return (
        <DropdownMenu.Root>
            <DropdownMenu.Trigger asChild>
                <button
                    className={classNames(
                        'flex items-center gap-2 px-3 py-1.5 text-sm rounded-md border border-secondary',
                        'bg-primary hover:bg-secondary data-[state=open]:bg-secondary transition-colors duration-200',
                        'focus:outline-none focus:ring-2 focus:ring-secondary/50'
                    )}
                >
                    {currentOption.icon}
                    <span>{currentOption.label}</span>
                    <FiChevronDown size={14} className="text-primary/60" />
                </button>
            </DropdownMenu.Trigger>

            <DropdownMenu.Portal>
                <DropdownMenu.Content
                    className={classNames(
                        'min-w-[140px] bg-primary border border-secondary rounded-md shadow-lg',
                        'p-1 z-50',
                        'animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
                        'data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2'
                    )}
                    sideOffset={4}
                    align="end"
                >
                    {themeOptions.map((option) => (
                        <DropdownMenu.Item
                            key={option.id}
                            className={classNames(
                                'flex items-center gap-2 px-3 py-2 text-sm rounded cursor-pointer',
                                'hover:bg-secondary focus:bg-secondary focus:outline-none',
                                'transition-colors duration-150',
                                currentTheme === option.id && 'bg-tertiary text-primary font-medium'
                            )}
                            onSelect={() => setTheme(option.id)}
                        >
                            {option.icon}
                            <span>{option.label}</span>
                        </DropdownMenu.Item>
                    ))}
                </DropdownMenu.Content>
            </DropdownMenu.Portal>
        </DropdownMenu.Root>
    );
};
