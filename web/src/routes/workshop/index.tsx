import { createFileRoute } from '@tanstack/react-router';
import { LuPencil } from 'react-icons/lu';

type Message = {
    role: 'user' | 'assistant';
    content: string;
};

type Chat = {
    title: string;
    messages: Message[];
    createdAt: string;
};

function useWorkshop(): { data: Chat } {
    return {
        data: {
            title: 'EIP-1337 Summary',
            messages: [
                {
                    role: 'user',
                    content: 'Can you summarize EIP-1337 for me?',
                },
                {
                    role: 'assistant',
                    content:
                        'EIP-1337 is a proposed standard for subscription payments on Ethereum. It introduces a new token standard that allows for recurring payments and subscriptions to be handled on-chain in a standardized way. The key features include:\n\n1. Automatic recurring payments\n2. Cancellation rights\n3. Pause/resume functionality\n4. Standardized interface for subscription management',
                },
                {
                    role: 'user',
                    content: 'How does it compare to other subscription payment solutions?',
                },
                {
                    role: 'assistant',
                    content:
                        'Compared to other solutions, EIP-1337 offers several advantages:\n\n1. On-chain standardization - unlike custom implementations, it provides a common interface that all dApps can use\n2. Better user experience - users can manage all their subscriptions in one place\n3. More secure - built-in cancellation and pause mechanisms protect users\n4. Interoperable - works across different dApps and platforms\n\nHowever, it does face some challenges:\n- Higher gas costs compared to off-chain solutions\n- Requires more complex smart contract implementation\n- Still needs wider adoption to reach its full potential',
                },
                {
                    role: 'user',
                    content: 'What are the main criticisms of the proposal?',
                },
                {
                    role: 'assistant',
                    content:
                        'The main criticisms of EIP-1337 include:\n\n1. Gas efficiency - some developers argue that subscription payments could be handled more efficiently off-chain\n2. Complexity - the standard adds another layer of complexity to the Ethereum ecosystem\n3. Adoption challenges - getting widespread adoption of a new token standard is difficult\n4. Competition - there are already several established solutions for subscription payments\n\nHowever, proponents argue that the benefits of standardization and on-chain management outweigh these concerns.',
                },
                {
                    role: 'user',
                    content: 'What are the main criticisms of the proposal?',
                },
                {
                    role: 'assistant',
                    content:
                        'The main criticisms of EIP-1337 include:\n\n1. Gas efficiency - some developers argue that subscription payments could be handled more efficiently off-chain\n2. Complexity - the standard adds another layer of complexity to the Ethereum ecosystem\n3. Adoption challenges - getting widespread adoption of a new token standard is difficult\n4. Competition - there are already several established solutions for subscription payments\n\nHowever, proponents argue that the benefits of standardization and on-chain management outweigh these concerns.',
                },
            ],
            createdAt: new Date().toISOString(),
        },
    };
}

export const Route = createFileRoute('/workshop/')({
    component: RouteComponent,
});

// function Header() {
//     return (
//         <div className="card flex-1 flex flex-col gap-1 h-fit col-span-full w-full">
//             <h1 className="">Ethereum AI Workshop</h1>
//             <p className="text-secondary">
//                 Query the Ethereum forums to find the best answers to your questions.
//             </p>
//         </div>
//     );
// }

function AssistantMessage({ message }: { message: Message }) {
    return (
        <div className="flex flex-col gap-2 rounded-lg bg-primary mr-8">
            <div className="whitespace-pre-wrap">{message.content}</div>
        </div>
    );
}

function UserMessage({ message }: { message: Message }) {
    return (
        <div className="flex flex-col p-4 rounded-lg bg-secondary ml-auto group relative">
            <div className="whitespace-pre-wrap">{message.content}</div>
            <div className="flex items-center justify-end gap-2 w-full opacity-0 group-hover:opacity-100 transition-opacity absolute -bottom-5 right-2">
                <button className="text-sm hover:text-primary">
                    <LuPencil className="size-4" />
                </button>
            </div>
        </div>
    );
}

function ChatMessage({ message }: { message: Message }) {
    return message.role === 'user' ? (
        <UserMessage message={message} />
    ) : (
        <AssistantMessage message={message} />
    );
}

function RouteComponent() {
    const { data } = useWorkshop();

    return (
        <>
            <div className="mx-auto max-w-screen-lg w-full pt-4 px-2 space-y-4 relative pb-12">
                {/* <Header /> */}

                <div className="text-lg font-bold border-b border-b-primary">{data.title}</div>

                <div className="flex flex-col gap-8">
                    {data.messages.map((message) => (
                        <ChatMessage key={message.content} message={message} />
                    ))}
                </div>
            </div>

            <div className="fixed bottom-0 left-1/2 w-full -translate-x-1/2 max-w-screen-lg bg-primary border-t border-primary/10 p-4">
                <div className="flex items-center gap-3">
                    <input
                        type="text"
                        // flex-1 bg-primary/5 border border-primary/10 rounded-lg px-4 py-2.5 focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-transparent transition-all
                        className="flex-1 bg-primary/5 border border-primary/10 rounded-lg px-4 py-2.5 focus:outline-none focus:ring-2 focus:ring-primary/20 focus:border-transparent transition-all"
                        placeholder="Ask anything..."
                    />
                    <button className="button bg-primary text-white hover:bg-primary/90 transition-colors px-6 py-2.5 rounded-lg font-medium">
                        Send
                    </button>
                </div>
            </div>
        </>
    );
}
