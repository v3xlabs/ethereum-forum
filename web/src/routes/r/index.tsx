import { createFileRoute } from '@tanstack/react-router';
import { useEffect } from 'react';

export const Route = createFileRoute('/r/')({
    component: RouteComponent,
    context: () => ({
        title: 'Roadmap',
    }),
});

function RouteComponent() {
    useEffect(() => {
        window.location.href = 'https://forkcast.org';
    }, []);

    return null;
}
