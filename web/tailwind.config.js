/** @type {import('tailwindcss').Config} */
export default {
    content: ['./src/**/*.{html,tsx}'],
    theme: {
        extend: {
            backgroundColor: {
                primary: 'rgb(var(--theme-bg-primary))',
                secondary: 'rgb(var(--theme-bg-secondary))',
                tertiary: 'rgb(var(--theme-bg-tertiary))',
            },
            textColor: {
                primary: 'rgb(var(--theme-text-primary))',
                secondary: 'rgb(var(--theme-text-secondary))',
            },
            borderColor: {
                primary: 'rgb(var(--theme-border))',
                secondary: 'rgb(var(--theme-border-secondary))',
            },
            outlineColor: {
                primary: 'rgb(var(--theme-border))',
                secondary: 'rgb(var(--theme-border-secondary))',
            },
            colors: {
                // background: 'var(--theme-bg-color)',
                // text: 'var(--theme-text-color)',
                // hint: 'var(--theme-hint-color)',
                // link: 'var(--theme-link-color)',
                // button: 'var(--theme-button-color)',
                // 'button-text': 'var(--theme-button-text-color)',
                // 'secondary-background': 'var(--theme-secondary-bg-color)',
                // 'header-background': 'var(--theme-header-bg-color)',
                // 'bottom-bar-background': 'var(--theme-bottom-bar-bg-color)',
                // 'accent-text-color': 'var(--theme-accent-text-color)',
                // 'section-background': 'var(--theme-section-bg-color)',
                // 'section-header-text': 'var(--theme-section-header-text-color)',
                // 'section-seperator': 'var(--theme-section-seperator-color)',
                // 'subtitle-text': 'var(--theme-subtitle-text-color)',
                // 'destructive-text': 'var(--theme-destructive-color)',
            },
            animation: {
                'spin-slow': 'spin 9s linear infinite',
                zoom: 'zoom 1s ease-in-out infinite',
                'pulse-ring': 'pulseRing 1.5s linear infinite',
                'pulse-ring-delay-1': 'pulseRing 1.5s linear infinite 0.5s',
                'pulse-ring-delay-2': 'pulseRing 1.5s linear infinite 1s',
            },
            keyframes: {
                pulseRing: {
                    '0%': { transform: 'scale(0.8)', opacity: 1 },
                    '70%': { transform: 'scale(1.8)', opacity: 0 },
                    '100%': { transform: 'scale(2)', opacity: 0 },
                },
            },
            screens: {
                '3xl': '1780px',
            },
        },
    },
    plugins: [
        function ({ addUtilities }) {
            const newUtilities = {
                '.animation-delay-0': {
                    'animation-delay': '0s',
                },
                '.animation-delay-300': {
                    'animation-delay': '0.3s',
                },
                '.animation-delay-600': {
                    'animation-delay': '0.6s',
                },
            };

            addUtilities(newUtilities);
        },
    ],
};
