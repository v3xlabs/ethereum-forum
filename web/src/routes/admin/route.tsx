import { createFileRoute, Link, Outlet } from '@tanstack/react-router';
import { useState } from 'react';

const navLinks = [
    { to: '/admin', label: 'Overview', exact: true },
    { to: '/admin/runs', label: 'Runs', exact: false },
    { to: '/admin/memory', label: 'Memory', exact: false },
    { to: '/admin/prompts', label: 'Prompts', exact: false },
    { to: '/admin/actions', label: 'Actions', exact: false },
] as const;

const AdminKeyPrompt = ({ onSubmit }: { onSubmit: (key: string) => void }) => (
    <div className="flex items-center justify-center min-h-[50vh]">
        <div className="max-w-sm w-full space-y-4">
            <h1 className="text-xl font-semibold">Admin Access</h1>
            <p className="text-sm text-primary/60">
                Enter your admin API key to access the admin panel.
            </p>
            <input
                type="password"
                placeholder="Admin API Key"
                className="w-full px-3 py-2 border border-primary/20 rounded bg-primary/5 text-sm"
                onKeyDown={(event) => {
                    if (event.key === 'Enter' && event.currentTarget.value) {
                        onSubmit(event.currentTarget.value);
                    }
                }}
            />
            <p className="text-xs text-primary/40">Press Enter to submit</p>
        </div>
    </div>
);

const AdminLayout = () => {
    const [adminKey, setAdminKey] = useState(() => localStorage.getItem('admin_key'));

    if (!adminKey) {
        return (
            <AdminKeyPrompt
                onSubmit={(key) => {
                    localStorage.setItem('admin_key', key);
                    setAdminKey(key);
                }}
            />
        );
    }

    return (
        <div className="py-6 space-y-6">
            <nav className="flex items-center gap-1 border-b border-primary/20 pb-2 overflow-x-auto">
                {navLinks.map(({ to, label, exact }) => (
                    <Link key={to} to={to} activeOptions={{ exact }}>
                        {({ isActive }) => (
                            <span
                                className={`block px-3 py-1.5 rounded text-sm whitespace-nowrap transition-colors ${
                                    isActive
                                        ? 'bg-primary/10 text-primary font-medium'
                                        : 'text-primary/60 hover:text-primary hover:bg-primary/5'
                                }`}
                            >
                                {label}
                            </span>
                        )}
                    </Link>
                ))}
            </nav>
            <Outlet />
        </div>
    );
};

export const Route = createFileRoute('/admin')({
    component: AdminLayout,
});
