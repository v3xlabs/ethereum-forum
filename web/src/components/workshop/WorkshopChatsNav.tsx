import * as Dialog from '@radix-ui/react-dialog';
import { Link, useNavigate, useParams, useRouterState } from '@tanstack/react-router';
import classNames from 'classnames';
import { FC, useState } from 'react';
import { FiBarChart } from 'react-icons/fi';
import { LuX } from 'react-icons/lu';

import { useAuth } from '@/api/auth';
import { useWorkshopChats, useWorkshopDeleteChat } from '@/api/workshop';

import { Tooltip } from '../tooltip/Tooltip';

export const WorkshopChatsNav = () => {
    const { isAuthenticated } = useAuth();

    // Don't render anything if not authenticated
    if (!isAuthenticated) {
        return null;
    }

    // Only render the component that uses the hook when authenticated
    return <AuthenticatedWorkshopChats />;
};

const AuthenticatedWorkshopChats = () => {
    const { data: chats } = useWorkshopChats();
    const { pathname } = useRouterState({ select: (s) => s.location });

    // Safely get chatId only if we're on a route that has it
    let chatId: string | undefined;

    try {
        const params = useParams({ from: '/chat/$chatId' });

        // eslint-disable-next-line prefer-destructuring
        chatId = params.chatId;
    } catch {
        // We're not on a /chat/$chatId route, so no chatId
        chatId = undefined;
    }

    return (
        <div className="max-h-[50vh] overflow-y-auto space-y-1">
            {/* Usage Statistics Link */}
            <div className="border-b border-primary pb-1 mb-2">
                <Link
                    to="/chat/usage"
                    className={classNames(
                        'flex items-center gap-2 hover:bg-secondary px-1.5 py-0.5 rounded text-sm text-secondary',
                        pathname === '/chat/usage' && 'bg-secondary'
                    )}
                >
                    <FiBarChart size={14} />
                    <span>Usage Statistics</span>
                </Link>
            </div>

            {/* Chat List */}
            <ul>
                {chats?.map((chat) => (
                    <li key={chat.chat_id} className="group/workshop">
                        <Link
                            to="/chat/$chatId"
                            params={{ chatId: chat.chat_id }}
                            hash={chat.last_message_id}
                            className={classNames(
                                'flex justify-between items-center hover:bg-tertiary px-1.5 py-0.5 relative group/workshoplink',
                                chat.chat_id === chatId && 'bg-tertiary'
                            )}
                        >
                            <div className="w-full">
                                <div className="absolute top-0 left-2 w-2 h-full border-l-2 border-primary group-last/workshop:h-1/2"></div>
                                <div className="absolute top-0 left-2 w-2 h-full border-primary border-b-2 -translate-y-1/2"></div>
                                <Tooltip
                                    trigger={
                                        <div className="pl-4 overflow-hidden text-ellipsis whitespace-nowrap w-full">
                                            {chat.summary || 'Untitled conversation'}
                                        </div>
                                    }
                                >
                                    {chat.summary || 'Untitled conversation'}
                                </Tooltip>
                                <div className="absolute right-0 top-1/2 -translate-y-1/2 hidden group-hover/workshoplink:block">
                                    <DeleteButton chatId={chat.chat_id} currentChatId={chatId} />
                                </div>
                            </div>
                        </Link>
                    </li>
                ))}
            </ul>
        </div>
    );
};

export const DeleteButton: FC<{ chatId: string; currentChatId?: string }> = ({
    chatId,
    currentChatId,
}) => {
    const { mutate: deleteChat } = useWorkshopDeleteChat();
    const navigate = useNavigate();
    const [open, setOpen] = useState(false);

    const handleDelete = () => {
        deleteChat(chatId, {
            onSuccess() {
                setOpen(false);

                if (chatId === currentChatId) {
                    navigate({ to: '/chat/$chatId', params: { chatId: 'new' } });
                }
            },
        });
    };

    return (
        <Dialog.Root open={open} onOpenChange={setOpen}>
            <Dialog.Trigger asChild>
                <button className="button flex items-center gap-2">
                    <LuX />
                </button>
            </Dialog.Trigger>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 z-40 data-[state=open]:animate-overlayShow overflow-y-scroll grid place-items-center">
                    <Dialog.Content className="z-50 relative my-10 max-w-md shadow-[var(--shadow-6)] focus:outline-none data-[state=open]:animate-contentShow mx-auto w-full p-6 bg-primary space-y-4">
                        <Dialog.Title className="text-xl font-bold">
                            Delete Conversation
                        </Dialog.Title>
                        <Dialog.Description>
                            Are you sure you want to delete this conversation? This action cannot be
                            undone.
                        </Dialog.Description>
                        <div className="flex gap-3 justify-end">
                            <Dialog.Close asChild>
                                <button className="px-4 py-2 text-sm font-medium button">
                                    Cancel
                                </button>
                            </Dialog.Close>
                            <button
                                onClick={handleDelete}
                                className="px-4 py-2 text-sm font-medium button button-red"
                            >
                                Delete
                            </button>
                        </div>
                        <Dialog.Close className="absolute top-2 right-2 -translate-y-1/2 hover:bg-secondary rounded-full p-1">
                            <LuX className="size-5" />
                        </Dialog.Close>
                    </Dialog.Content>
                </Dialog.Overlay>
            </Dialog.Portal>
        </Dialog.Root>
    );
};
