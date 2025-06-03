import { createFileRoute } from '@tanstack/react-router';
import { FC } from 'react';

import { useUser } from '@/api/user';

const RouteComponent: FC = () => {
    const { userId } = Route.useParams();

    const { data: userData, isLoading } = useUser(userId);

    if (isLoading) {
        return (
            <div className="mx-auto w-full max-w-screen-lg pt-8 px-2">
                <h1 className="text-3xl">Loading...</h1>
            </div>
        );
    }

    if (!userData) {
        return (
            <div className="mx-auto w-full max-w-screen-lg pt-8 px-2">
                <h1 className="text-3xl">User not found</h1>
            </div>
        );
    }

    const hasVanityName =
        userData.user.name &&
        userData.user.name.toLowerCase() !== userData.user.username.toLowerCase();

    return (
        <div className="mx-auto w-full max-w-screen-lg px-2 space-y-6">
            <div className="flex items-center gap-6 py-4">
                <div className="size-20 rounded-full overflow-hidden border-4 border-gray-200 shadow">
                    <img
                        src={
                            'https://ethereum-magicians.org' +
                            userData.user.avatar_template.replace('{size}', '200')
                        }
                        alt={`${userId} avatar`}
                        className="object-cover w-full h-full"
                    />
                </div>
                <div className="flex flex-col justify-center">
                    <h1 className="text-3xl">
                        {hasVanityName ? userData.user.name : userData.user.username}
                    </h1>
                    {hasVanityName && (
                        <span className="text-secondary">@{userData.user.username}</span>
                    )}
                </div>
            </div>

            <div className="flex flex-wrap text-sm text-primary border-b pb-2 border-primary gap-x-6 font-thin">
                <span>
                    Joined: <span className="font-semibold">{userData.user.created_at}</span>
                </span>
                <span>
                    Last post: <span className="font-semibold">{userData.user.last_posted_at}</span>
                </span>
            </div>

            <pre>{JSON.stringify(userData, undefined, 2)}</pre>
        </div>
    );
};

export const Route = createFileRoute('/u/$userId/')({
    component: RouteComponent,
});
