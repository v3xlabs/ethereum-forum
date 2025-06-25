import { createRootRoute, Outlet } from '@tanstack/react-router';
import { Toaster } from 'sonner';

import { CommandMenu } from '@/components/command/CommandMenu';
import { Navbar } from '@/components/Navbar';
import { Sidebar } from '@/components/Sidebar';
import { AppWrapper } from '@/hooks/context';

export const Route = createRootRoute({
    component: RootComponent,
});

function RootComponent() {
    return (
        <AppWrapper>
            <CommandMenu />
            <div className="flex">
                <Sidebar />
                <div className="max-w-screen w-full relative">
                    <Navbar />
                    <div className="px-2">
                        <Outlet />
                    </div>
                </div>
            </div>
            <Toaster
                expand={true}
                visibleToasts={9}
                mobileOffset={{ top: 48 }}
                toastOptions={{
                    style: {
                        backgroundColor: 'rgb(var(--theme-bg-primary))',
                        color: 'rgb(var(--theme-text-primary))',
                        border: '1px solid rgb(var(--theme-border-secondary))',
                        borderRadius: '0.375rem',
                        padding: '0.75rem 1rem',
                        fontSize: '0.875rem',
                        fontWeight: '500',
                        boxShadow:
                            '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
                    },
                    classNames: {
                        title: 'font-bold',
                        description: 'opacity-90',
                        actionButton: 'rounded px-2 py-1 text-sm font-medium transition-colors',
                        cancelButton: 'rounded px-2 py-1 text-sm font-medium transition-colors',
                        closeButton: 'rounded px-1 py-1 text-sm font-medium transition-colors',
                        error: 'border-red-500 text-red-600',
                        success: 'border-green-500 text-green-600',
                        warning: 'border-yellow-500 text-yellow-600',
                        info: 'border-blue-500 text-blue-600',
                    },
                }}
            />
        </AppWrapper>
    );
}
