import { createFileRoute, useNavigate, useParams } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { LuArrowRight, LuShare } from 'react-icons/lu';
import { match, P } from 'ts-pattern';

import { getWorkshopChat, useWorkshopChat, useWorkshopSendMessage } from '@/api/workshop';
import { UpDownScroller } from '@/components/UpDown';
import { ChatMessage } from '@/components/workshop/ChatMessage';
import { queryClient } from '@/util/query';

const suggestions = [
    // eslint-disable-next-line quotes
    "Evaluate vitalik's opinion on RISC-V within the EVM",
    'Summarize EIP-7702, who it affects, and what I can do to understand it better',
    'What is currently being talked about?',
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
    const [placeholder, setPlaceholder] = useState(true);
    const { chatId } = useParams({ from: '/chat/$chatId' });

    // if the user types "unlock", set the placeholder to false
    useEffect(() => {
        let lastKeys = '';
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key) {
                lastKeys += e.key;
                lastKeys = lastKeys.slice(-6);

                if (lastKeys === 'unlock') {
                    setPlaceholder(false);
                    document.removeEventListener('keydown', handleKeyDown);
                }
            }
        };

        document.addEventListener('keydown', handleKeyDown);

        return () => {
            document.removeEventListener('keydown', handleKeyDown);
        };
    }, []);

    return (
        <div className="mx-auto w-full max-w-screen-lg pt-8 px-2 space-y-4">
            {placeholder && <Placeholder />}
            {!placeholder && <Chat chatId={chatId} />}
        </div>
    );
}

const Placeholder = () => {
    return (
        <>
            <div>Welcome to the workshop!</div>
            <div className="card space-y-2">
                <p>The workshop is currently in beta and not publicly accessible yet.</p>
                <p>Please check back later.</p>
            </div>
        </>
    );
};

const Chat = ({ chatId }: { chatId: string }) => {
    const { data: chat } = useWorkshopChat(chatId);
    const [input, setInput] = useState('');
    const lastChatMessage = chat?.messages?.[chat?.messages.length - 1];
    const { mutate: sendMessage } = useWorkshopSendMessage(chatId);
    const navigate = useNavigate();
    const onMessageSend = (message: string) => {
        sendMessage(
            { message, parent_message: lastChatMessage?.message_id },
            {
                onSuccess(data, variables) {
                    setInput('');

                    if (variables.parent_message === undefined) {
                        navigate({ to: '/chat/$chatId', params: { chatId: data.chat_id } });
                    }
                },
            }
        );
    };

    const greeting =
        new Date().getHours() < 12
            ? 'Good Morning'
            : new Date().getHours() < 18
              ? 'Good Afternoon'
              : 'Good Evening';

    return (
        <div className="w-full h-full relative py-1">
            <div className="w-full">
                <div className="relative h-fit">
                    {match(chat?.messages?.length)
                        .with(P.number.gt(0), () => (
                            <>
                                <UpDownScroller />
                                <div className="flex w-full justify-between items-center mb-4">
                                    <div>
                                        <h1 className="text-base">Untitled conversation</h1>
                                    </div>
                                    <div className="text-xs flex items-center gap-2">
                                        <button className="button flex items-center gap-2">
                                            <LuShare />
                                            Share
                                        </button>
                                    </div>
                                </div>
                                <div className="space-y-2 pb-80 relative">
                                    {chat?.messages?.map((message) => (
                                        <ChatMessage key={message.message_id} message={message} />
                                    ))}
                                </div>
                                <div className="w-full fixed max-w-screen-lg bottom-0 inset-x-0 mx-auto">
                                    <InputBox
                                        input={input}
                                        setInput={setInput}
                                        onSend={onMessageSend}
                                    />
                                    <div className="text-center text-sm py-1">
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
                                                    className="button button-primary whitespace-nowrap max-w-64 overflow-hidden text-ellipsis"
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
}: {
    input: string;
    setInput: (input: string) => void;
    onSend: (input: string) => void;
}) => {
    return (
        <div className="w-full relative">
            <textarea
                name="chatbox"
                id="chatbox"
                placeholder="Type your message here..."
                className="w-full h-full bg-primary border-primary/50 border rounded-md p-3 focus:border-primary focus:ring-primary/50 focus:ring-2 outline-none max-h-80 min-h-32"
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
            >
                <LuArrowRight />
            </button>
        </div>
    );
};
