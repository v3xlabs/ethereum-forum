import { createFileRoute, useLocation, useNavigate, useParams } from '@tanstack/react-router';
import { FC, useEffect, useMemo, useState } from 'react';
import { LuArrowRight, LuListTree, LuLoader, LuShare } from 'react-icons/lu';
import { match, P } from 'ts-pattern';

import {
    getWorkshopChat,
    useWorkshopChat,
    useWorkshopChatShare,
    useWorkshopSendMessage,
    WorkshopMessage,
} from '@/api/workshop';
import { WorkshopAuthGuard } from '@/components/AuthGuard';
import { UpDownScroller } from '@/components/UpDown';
import { ChatMessage, convertToExtendedMessage } from '@/components/workshop/ChatMessage';
import { ConversationGraph, ConversationGraphProps } from '@/components/workshop/ConversationGraph';
import { ModelSelector } from '@/components/workshop/ModelSelector';
import {
    buildMessageTree,
    buildPathToMessage,
    getVisiblePath,
    MessagePath,
    updatePath,
} from '@/util/messageTree';
import { queryClient } from '@/util/query';

const suggestions = [
    'Find topics related to risc-v in the evm and evaluate me the opinions of all parties involved.',
    'Summarize EIP-7702, who it affects, and what I can do to understand it better',
    'What are the main concerns with EIP-4844 blob transactions according to the community?',
    'Find discussions about MEV and explain the current consensus on solutions',
    // 'Compare community opinions on different Layer 2 scaling approaches',
    'Analyze the debate around account abstraction implementations',
    'What are researchers saying about zkEVM developments?',
    // 'Find all discussions about gas optimization techniques in smart contracts',
];

const isUuid = (value: string) => {
    return /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/.test(
        value
    );
};

export const Route = createFileRoute('/chat/$chatId')({
    component: RouteComponent,
    context: () => ({
        title: 'Workshop',
    }),
    async beforeLoad(ctx) {
        const { chatId } = ctx.params;

        if (chatId !== 'new' && isUuid(chatId)) {
            const chat = await queryClient.ensureQueryData(getWorkshopChat(chatId));

            return { title: chat?.chat?.summary ?? 'Untitled conversation' };
        }

        return { title: 'New chat' };
    },
});

function RouteComponent() {
    const { chatId } = useParams({ from: '/chat/$chatId' });

    return (
        <>
            <div className="mx-auto w-full prose-width pt-8 px-2 space-y-4">
                <WorkshopAuthGuard>
                    <ChatWithSidebar chatId={chatId} />
                </WorkshopAuthGuard>
            </div>
        </>
    );
}

const ChatWithSidebar = ({ chatId }: { chatId: string }) => {
    const { data: chat } = useWorkshopChat(chatId);
    const [messagePath, setMessagePath] = useState<MessagePath>({});
    const useTreeView = true;

    // Build message tree - memoized to prevent infinite loops
    const { rootNodes, messageMap } = useMemo(() => {
        return buildMessageTree(chat?.messages || []);
    }, [chat?.messages]);

    // Initialize path based on last_message_id when chat data changes
    useEffect(() => {
        if (chat?.chat?.last_message_id && messageMap.size > 0) {
            const initialPath = buildPathToMessage(messageMap, chat.chat.last_message_id);

            // Also set the root selection - find which root message leads to the last message
            const lastMessage = messageMap.get(chat.chat.last_message_id);

            if (lastMessage) {
                // Trace back to find the root message
                let currentMessage = lastMessage.message;

                while (currentMessage.parent_message_id) {
                    const parentNode = messageMap.get(currentMessage.parent_message_id);

                    if (parentNode) {
                        currentMessage = parentNode.message;
                    } else {
                        break;
                    }
                }
                // currentMessage should now be the root message
                initialPath['root'] = currentMessage.message_id;
            }

            setMessagePath(initialPath);
        }
    }, [chat?.chat?.last_message_id, messageMap.size]);

    const visibleMessages = useMemo(() => {
        return getVisiblePath(rootNodes, messagePath);
    }, [rootNodes, messagePath]);

    const handleNavigateToMessage = (message: WorkshopMessage) => {
        if (message.parent_message_id) {
            // Regular case: message has a parent, update the path normally
            const newPath = updatePath(messagePath, message.parent_message_id, message.message_id);

            setMessagePath(newPath);
        } else {
            // Special case: root-level message (no parent), use 'root' as the parent key
            const newPath = updatePath(messagePath, 'root', message.message_id);

            setMessagePath(newPath);
        }
    };

    return (
        <>
            {rootNodes.length > 1 && visibleMessages.length > 1 && messageMap && (
                <MessageTreeButton
                    rootNodes={rootNodes}
                    visibleMessages={visibleMessages}
                    messageMap={messageMap}
                />
            )}
            {/* Main Chat */}
            <Chat
                chatId={chatId}
                chat={chat}
                visibleMessages={visibleMessages}
                useTreeView={useTreeView}
                onNavigateToMessage={handleNavigateToMessage}
            />
        </>
    );
};

const Chat = ({
    chatId,
    chat,
    visibleMessages,
    useTreeView,
    onNavigateToMessage,
}: {
    chatId: string;
    chat: any;
    visibleMessages: any[];
    useTreeView: boolean;
    onNavigateToMessage: (message: WorkshopMessage) => void;
}) => {
    const [input, setInput] = useState('');
    const [editingMessage, setEditingMessage] = useState<WorkshopMessage | null>(null);
    const [selectedModel, setSelectedModel] = useState<string>('');
    const { mutate: sendMessage, isPending: sending } = useWorkshopSendMessage(chatId);
    const navigate = useNavigate();
    const { hash } = useLocation();

    // Load selected model from localStorage on mount
    useEffect(() => {
        const savedModel = localStorage.getItem('workshop_selected_model');

        if (savedModel) {
            setSelectedModel(savedModel);
        }
    }, []);

    // Save selected model to localStorage when it changes
    const handleModelChange = (modelId: string) => {
        setSelectedModel(modelId);
        localStorage.setItem('workshop_selected_model', modelId);
    };

    // Get the last message from the currently visible branch (for replying)
    const lastVisibleMessage =
        visibleMessages.length > 0
            ? visibleMessages[visibleMessages.length - 1].message
            : chat?.messages?.[chat?.messages.length - 1]; // Fallback to globally last message

    // Debug logging - only log once when messages actually change
    useEffect(() => {
        if (visibleMessages.length > 0) {
            // Debug info for development - can be removed later
            // console.log('Visible messages updated', visibleMessages.length);
        }
    }, [chat?.messages?.length]); // Only log when message count changes

    // Handle scrolling to hash element (last message)
    useEffect(() => {
        if (hash && chat?.messages?.length) {
            // Use setTimeout to ensure the DOM is fully rendered
            setTimeout(() => {
                const element = document.getElementById(hash);

                if (element) {
                    element.scrollIntoView({
                        behavior: 'smooth',
                    });
                }
            }, 100);
        }
    }, [chatId, hash]);

    const onMessageSend = (message: string) => {
        const parentMessageId = editingMessage
            ? editingMessage.parent_message_id
            : lastVisibleMessage?.message_id;

        sendMessage(
            {
                message,
                parent_message: parentMessageId,
                model: selectedModel || undefined,
            },
            {
                onSuccess(data, variables) {
                    setInput('');
                    setEditingMessage(null);

                    if (variables.parent_message === undefined) {
                        navigate({ to: '/chat/$chatId', params: { chatId: data.chat_id } });
                    }
                },
            }
        );
    };

    const handleEditMessage = (message: WorkshopMessage) => {
        setInput(message.message);
        setEditingMessage(message);
    };

    const cancelEdit = () => {
        setInput('');
        setEditingMessage(null);
    };

    const greeting =
        new Date().getHours() < 12
            ? 'Good Morning'
            : new Date().getHours() < 18
              ? 'Good Afternoon'
              : 'Good Evening';

    // Determine which messages to show
    const messageCount = useTreeView ? visibleMessages.length : chat?.messages?.length || 0;

    return (
        <div className="w-full h-full relative py-1">
            <div className="w-full">
                <div className="relative h-fit">
                    {match(messageCount)
                        .with(P.number.gt(0), () => (
                            <>
                                <UpDownScroller />
                                <div className="flex w-full justify-between items-center mb-4">
                                    <div>
                                        <h1 className="text-base font-bold">
                                            {chat?.chat?.summary || 'Untitled conversation'}
                                        </h1>
                                    </div>
                                    <div className="text-xs flex items-center gap-2">
                                        {/* <button
                                            className="button text-xs"
                                            onClick={() => setUseTreeView(!useTreeView)}
                                            title={
                                                useTreeView ? 'Show all messages' : 'Show tree view'
                                            }
                                        >
                                            {useTreeView ? 'All' : 'Tree'}
                                        </button> */}
                                        <ShareButton
                                            chatId={chatId}
                                            messageId={lastVisibleMessage?.message_id ?? ''}
                                        />
                                    </div>
                                </div>
                                <div className="space-y-2 pb-80 relative">
                                    {useTreeView
                                        ? visibleMessages.map((node) => (
                                              <ChatMessage
                                                  key={node.message.message_id}
                                                  node={node}
                                                  editable={true}
                                                  onEdit={handleEditMessage}
                                                  onNavigate={onNavigateToMessage}
                                              />
                                          ))
                                        : chat?.messages?.map((message: WorkshopMessage) => (
                                              <ChatMessage
                                                  key={message.message_id}
                                                  message={convertToExtendedMessage(message)}
                                                  editable={true}
                                                  onEdit={handleEditMessage}
                                              />
                                          ))}
                                </div>
                                <div className="w-full sticky prose-width px-4 bottom-2 inset-x-0 mx-auto">
                                    <InputBox
                                        input={input}
                                        setInput={setInput}
                                        onSend={onMessageSend}
                                        sending={sending}
                                        editingMessage={editingMessage}
                                        onCancelEdit={cancelEdit}
                                        selectedModel={selectedModel}
                                        onModelChange={handleModelChange}
                                    />
                                    <div className="text-center text-sm py-1 hidden md:block">
                                        This is a demo. Check important info.
                                    </div>
                                </div>
                            </>
                        ))
                        .otherwise(() => (
                            <div className="w-full h-fit pt-8 md:py-64">
                                <div className="w-full max-w-screen-md mx-auto space-y-4">
                                    <h2 className="text-center text-2xl font-bold">
                                        {greeting}, Ready to research?
                                    </h2>
                                    {suggestions.length > 0 && input.length === 0 && (
                                        <div className="mx-auto flex justify-center gap-2 flex-wrap">
                                            {suggestions.map((suggestion) => (
                                                <button
                                                    key={suggestion}
                                                    className="button button-primary whitespace-wrap max-w-64 md:max-w-72 md:px-2 md:py-1 max-h-24 line-clamp-2 h-12 text-ellipsis text-start text-sm flex justify-start"
                                                    onClick={() => setInput(suggestion)}
                                                >
                                                    {suggestion}
                                                </button>
                                            ))}
                                        </div>
                                    )}
                                    <div className="w-full max-w-screen-md mx-auto">
                                        <InputBox
                                            input={input}
                                            setInput={setInput}
                                            onSend={onMessageSend}
                                            sending={sending}
                                            editingMessage={null}
                                            onCancelEdit={cancelEdit}
                                            selectedModel={selectedModel}
                                            onModelChange={handleModelChange}
                                        />
                                    </div>
                                </div>
                            </div>
                        ))}
                </div>
            </div>
        </div>
    );
};

const InputBox = ({
    input,
    setInput,
    onSend,
    sending,
    editingMessage,
    onCancelEdit,
    selectedModel,
    onModelChange,
}: {
    input: string;
    setInput: (input: string) => void;
    onSend: (input: string) => void;
    sending: boolean;
    editingMessage?: WorkshopMessage | null;
    onCancelEdit?: () => void;
    selectedModel?: string;
    onModelChange?: (modelId: string) => void;
}) => {
    return (
        <div className="w-full h-fit relative">
            {editingMessage && (
                <div className="bg-yellow-100 border border-yellow-300 rounded-md p-2 text-sm">
                    <div className="flex justify-between items-center">
                        <span>Editing message - this will create a new branch</span>
                        <button
                            onClick={onCancelEdit}
                            className="text-yellow-700 hover:text-yellow-900"
                        >
                            Cancel
                        </button>
                    </div>
                </div>
            )}
            {onModelChange && (
                <div className="mb-2">
                    <ModelSelector
                        selectedModel={selectedModel}
                        onModelChange={onModelChange}
                        className="w-fit"
                    />
                </div>
            )}
            <textarea
                name="chatbox"
                id="chatbox"
                placeholder="Type your message here..."
                className="w-full h-full bg-primary border-primary/50 border rounded-md p-3 focus:border-primary focus:ring-primary/50 focus:ring-2 outline-none max-h-80 min-h-32 block"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) {
                        e.preventDefault();
                        onSend(input);
                    }
                }}
            ></textarea>
            <button
                className="button button-primary absolute right-3 bottom-4 aspect-square size-8 flex items-center justify-center"
                onClick={() => onSend(input)}
                disabled={sending}
            >
                {sending ? <LuLoader className="animate-spin" /> : <LuArrowRight />}
            </button>
        </div>
    );
};

export const ShareButton: FC<{ chatId: string; messageId: string }> = ({ chatId, messageId }) => {
    const { mutate: shareChat } = useWorkshopChatShare();
    const navigate = useNavigate();

    return (
        <button
            className="button flex items-center gap-2"
            onClick={() =>
                shareChat(
                    { chatId, messageId },
                    {
                        onSuccess(data) {
                            navigate({
                                to: '/chat/share/$snapshotId',
                                params: { snapshotId: data.snapshot_id.toString() },
                            });
                        },
                    }
                )
            }
        >
            <LuShare />
            Share
        </button>
    );
};

export const MessageTreeButton: FC<ConversationGraphProps> = ({
    rootNodes,
    visibleMessages,
    messageMap,
}) => {
    const [isOpen, setIsOpen] = useState(false);

    return (
        <div className="relative">
            <button className="button flex items-center gap-2" onClick={() => setIsOpen(!isOpen)}>
                <LuListTree />
                Message Tree
            </button>
            {isOpen && (
                <div className="right-bar p-4">
                    <ConversationGraph
                        rootNodes={rootNodes}
                        visibleMessages={visibleMessages}
                        messageMap={messageMap}
                    />
                </div>
            )}
        </div>
    );
};
