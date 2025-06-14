/* eslint-disable react/prop-types */
import { Link } from '@tanstack/react-router';

import { useUser } from '@/api';
import { components } from '@/api/schema.gen';
import { mapDiscourseInstanceUrl } from '@/util/discourse';

import { CommandGroup, CommandItem } from '../Command';
import { useCommand } from '../CommandMenu';

const User: React.FC<{
    user: components['schemas']['DiscourseDetailedUser'];
    instance: string;
}> = ({ user, instance }) => {
    const renderAvatarTemplate = (template: string, size: number, instance: string) => {
        return `${mapDiscourseInstanceUrl(instance)}/${template.replace('{size}', String(size))}`;
    };

    const { handleClose } = useCommand();

    return (
        <Link
            to={'/u/$discourseId/$userId'}
            params={{ discourseId: instance, userId: user.username }}
            onClick={() => {
                handleClose();
            }}
        >
            <CommandItem
                className="flex flex-col items-start gap-2 px-3 py-1.5 data-[selected=true]:bg-secondary data-[selected=true]:text-primary rounded-md"
                key={user.id}
                value={`${instance}/${user.username}`}
            >
                <div className="flex items-center gap-2 mb-1">
                    {user.avatar_template && (
                        <img
                            src={renderAvatarTemplate(user.avatar_template, 24, instance)}
                            alt={user.username}
                            className="w-6 h-6 rounded-full"
                        />
                    )}
                    <span className="text-sm font-medium">{user.username}</span>
                </div>
            </CommandItem>
        </Link>
    );
};

export const Username: React.FC = () => {
    const { search } = useCommand();

    const { data: magicians } = useUser('magicians', search);
    const { data: research } = useUser('research', search);

    if (!magicians || !research) {
        return null;
    }

    return (
        <>
            <CommandGroup heading="Users">
                {magicians?.user && <User user={magicians.user} instance="magicians" />}
                {research?.user && <User user={research.user} instance="research" />}
            </CommandGroup>
        </>
    );
};
