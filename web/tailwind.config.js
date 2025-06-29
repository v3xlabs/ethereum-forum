/** @type {import('tailwindcss').Config} */
export default {
    content: ['./src/**/*.{html,tsx,ts,css}'],
    theme: {
        extend: {
            backgroundColor: {
                primary: 'rgb(var(--theme-bg-primary) / <alpha-value>) /* rgb(253, 246, 227) */',
                secondary:
                    'rgb(var(--theme-bg-secondary) / <alpha-value>) /* rgb(238, 232, 213) */',
                tertiary: 'rgb(var(--theme-bg-tertiary) / <alpha-value>) /* rgb(204, 192, 157) */',
                purple: 'rgb(var(--theme-bg-purple) / <alpha-value>) /* rgb(129, 52, 255) */',
                grey: 'rgb(var(--theme-border) / <alpha-value>) /* rgb(101, 123, 131) */',
            },
            textColor: {
                primary: 'rgb(var(--theme-text-primary) / <alpha-value>) /* rgb(101, 123, 131) */',
                secondary:
                    'rgb(var(--theme-text-secondary) / <alpha-value>) /* rgb(38, 139, 210) */',
                purple: 'rgb(var(--theme-text-purple) / <alpha-value>) /* rgb(103, 40, 185) */',
            },
            borderColor: {
                primary: 'rgb(var(--theme-border) / <alpha-value>) /* rgb(101, 123, 131) */',
                secondary:
                    'rgb(var(--theme-border-secondary) / <alpha-value>) /* rgb(238, 232, 213) */',
                purple: 'rgb(var(--theme-text-purple) / <alpha-value>) /* rgb(103, 40, 185) */',
            },
            ringColor: {
                primary: 'rgb(var(--theme-border) / <alpha-value>) /* rgb(101, 123, 131) */',
                secondary:
                    'rgb(var(--theme-border-secondary) / <alpha-value>) /* rgb(238, 232, 213) */',
            },
            outlineColor: {
                primary: 'rgb(var(--theme-border) / <alpha-value>) /* rgb(101, 123, 131) */',
                secondary:
                    'rgb(var(--theme-border-secondary) / <alpha-value>) /* rgb(238, 232, 213) */',
            },
            colors: {
                success: 'rgb(var(--theme-success) / <alpha-value>) /* rgb(16, 185, 129) */',
                error: 'rgb(var(--theme-error) / <alpha-value>) /* rgb(239, 68, 68) */',
                warning: 'rgb(var(--theme-warning) / <alpha-value>) /* rgb(234, 179, 8) */',
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
                overlayShow: 'overlayShow 250ms cubic-bezier(0.16, 1, 0.3, 1)',
                contentShow: 'contentShow 250ms cubic-bezier(0.16, 1, 0.3, 1)',
            },
            keyframes: {
                pulseRing: {
                    '0%': { transform: 'scale(0.8)', opacity: 1 },
                    '70%': { transform: 'scale(1.8)', opacity: 0 },
                    '100%': { transform: 'scale(2)', opacity: 0 },
                },
                overlayShow: {
                    from: { opacity: '0' },
                    to: { opacity: '1' },
                },
                contentShow: {
                    from: {
                        opacity: '0',
                        transform: 'translate(0, -4%) scale(0.96)',
                    },
                    to: { opacity: '1', transform: 'translate(0, 0) scale(1)' },
                },
            },

            screens: {
                '3xl': '1666px',
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
