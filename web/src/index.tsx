import './styles/index.css';
import './styles/themes.css';

import { createRouter, RouterProvider } from '@tanstack/react-router';
import React from 'react';
import ReactDOM from 'react-dom/client';

// Import the generated route tree
import { routeTree } from './routeTree.gen';
import { QueryProvider } from './util/query';

// Create a new router instance
const router = createRouter({
    routeTree,
    // defaultPendingComponent,
    // defaultErrorComponent: PageErrorBoundary,
    context: {
        title: 'Ethereum Forum',
    },
});

// Register the router instance for type safety
declare module '@tanstack/react-router' {
    interface Register {
        router: typeof router;
    }
}

// preflightAuth();

ReactDOM.createRoot(document.querySelector('#root')!).render(
    <React.StrictMode>
        <QueryProvider>
            <RouterProvider router={router} />
        </QueryProvider>
    </React.StrictMode>
);
