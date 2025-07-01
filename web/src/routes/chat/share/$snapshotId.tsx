import { createFileRoute } from '@tanstack/react-router';

import { useForumUser, useWorkshopSnapshot } from '@/api';
import { TimeAgo } from '@/components/TimeAgo';
import { ChatMessage } from '@/components/workshop/ChatMessage';

export const Route = createFileRoute('/chat/share/$snapshotId')({
    component: RouteComponent,
});

function RouteComponent() {
    const { snapshotId } = Route.useParams();
    const { data: snapshot } = useWorkshopSnapshot(snapshotId ?? '');
    const { data: by } = useForumUser(snapshot?.snapshot.user_id ?? undefined);

    return (
        <div className="mx-auto prose-width my-8">
            {typeof by === 'string' && (
                <div className="text-sm text-secondary mb-4">
                    Shared by <span className="font-semibold">&quot;{by}&quot;</span>{' '}
                    {snapshot?.snapshot.created_at && (
                        <TimeAgo date={new Date(snapshot?.snapshot.created_at)} />
                    )}
                </div>
            )}
            <div>
                <ul className="space-y-4">
                    {snapshot?.messages?.map((msg) => (
                        <ChatMessage
                            user={by || 'User'}
                            key={msg.message_id}
                            message={msg as any}
                        />
                    ))}
                </ul>
            </div>
        </div>
    );
}
