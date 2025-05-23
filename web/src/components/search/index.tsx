import { useTopic } from '@/api/topics';
import * as Dialog from '@radix-ui/react-dialog';
import { MagnifyingGlassIcon } from '@radix-ui/react-icons';
import { MeiliSearch } from 'meilisearch';
import { useEffect, useState } from 'react';
import { TopicPost } from '../topic/TopicPost';

const client = new MeiliSearch({
    host: 'http://localhost:7700',
    apiKey: 'masterKey',
});

const MEILISEARCH_INDEX_NAME = 'forum';

type ForumSearchDocument = {
    id: string;
    type_field: string;
    topic_id?: number;
    post_id?: number;
    post_number?: number;
    user_id?: number;
    title?: string;
    slug?: string;
    pm_issue?: number;
    cooked?: string;
};

type TopicSearchResult = ForumSearchDocument & {
    type_field: 'topic';
    topic_id: number;
    title: string;
    slug: string;
    pm_issue?: number;
    post_id?: never;
    post_number?: never;
    user_id?: never;
    cooked?: never;
};

type PostSearchResult = ForumSearchDocument & {
    type_field: 'post';
    topic_id: number;
    post_id: number;
    post_number: number;
    user_id: number;
    cooked?: string;
    title?: never;
    slug?: never;
    pm_issue?: never;
};

type SearchResult = TopicSearchResult | PostSearchResult;

const SelectedResult = ({ result }: { result: SearchResult }) => {
    const { data: topic, isLoading } = useTopic(result.topic_id.toString());

    if (isLoading) {
        return <p>Loading...</p>;
    }

    if (!topic) {
        return <p>Topic not found.</p>;
    }

    if (result.type_field === 'topic') {
        return (
            <div>
                <h2 className="text-2xl font-bold mb-2">{result.title}</h2>
            </div>
        );
    }

    if (result.type_field === 'post') {
        const extra = (topic.extra || {}) as Record<string, unknown>;
        const details = extra.details as Record<string, unknown> | undefined;
        const participants = details?.participants as
            | Array<{
                  id: number;
                  name: string;
                  username: string;
                  avatar_template: string;
              }>
            | undefined;
        const participant = participants?.find((p) => p.id === result.user_id);

        return (
            <div>
                <h2 className="text-2xl font-bold mb-2">{topic.title}</h2>
                <a href={'/t/' + result.topic_id + '#p-' + result.post_id}>View context</a>
                <TopicPost
                    post={{
                        post_id: result.post_id,
                        post_number: result.post_number,
                        user_id: result.user_id,
                        topic_id: result.topic_id,
                        cooked: result.cooked,
                        extra: {
                            display_username: participant?.name,
                            username: participant?.username,
                            avatar_template: participant?.avatar_template,
                            post_url: '',
                            hidden: false,
                            trust_level: 0,
                            moderator: false,
                            admin: false,
                        },
                    }}
                />
            </div>
        );
    }

    return null;
};

export const SearchBar = () => {
    const [searchTerm, setSearchTerm] = useState('');
    const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [selectedResult, setSelectedResult] = useState<SearchResult | null>(null);

    useEffect(() => {
        if (!isModalOpen) {
            setSearchTerm('');
            setSearchResults([]);
            setSelectedResult(null);

            return;
        }

        if (searchTerm.trim() === '') {
            setSearchResults([]);

            return;
        }

        const search = async () => {
            try {
                const index = client.index(MEILISEARCH_INDEX_NAME);
                const result = await index.search(searchTerm, {
                    attributesToHighlight: ['title', 'cooked'],
                });

                setSearchResults(result.hits as SearchResult[]);
            } catch (error) {
                console.error('MeiliSearch error:', error);
                setSearchResults([]);
            }
        };

        const debounceTimeout = setTimeout(() => {
            search();
        }, 300);

        return () => clearTimeout(debounceTimeout);
    }, [searchTerm, isModalOpen]);

    const handleResultClick = (result: SearchResult) => {
        setSelectedResult(result);
    };

    return (
        <Dialog.Root open={isModalOpen} onOpenChange={setIsModalOpen}>
            <Dialog.Trigger asChild>
                <button className="px-3 py-2 rounded-md cursor-pointer flex items-center">
                    <MagnifyingGlassIcon className="mr-2" />
                    Search
                </button>
            </Dialog.Trigger>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 z-20" />
                <Dialog.Content className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-primary rounded-lg shadow-lg p-5 w-[90vw] max-w-4xl h-[80vh] max-h-[600px] flex flex-col z-30">
                    <Dialog.Title className="text-xl font-bold mb-4">Search Forum</Dialog.Title>
                    <input
                        type="text"
                        placeholder="Search..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        className="px-3 py-2 mb-4 border border-gray-300 rounded-md text-base w-full"
                    />
                    <div className="flex flex-grow overflow-hidden border border-gray-200 rounded-md">
                        <div className="w-2/5 border-r border-gray-200 overflow-y-auto p-2.5 flex flex-col space-y-2">
                            {searchResults.length === 0 && searchTerm.trim() !== '' && (
                                <p className="text-gray-500">No results found.</p>
                            )}
                            {searchResults.map((result) => (
                                <div
                                    key={result.id}
                                    onClick={() => handleResultClick(result)}
                                    className={`border p-3 rounded-md cursor-pointer transition-colors duration-200 ease-in-out outline-offset-[-2px] ${
                                        selectedResult?.id === result.id
                                            ? 'border-transparent outline outline-2 outline-blue-500 bg-blue-50'
                                            : 'border-gray-300 bg-white hover:bg-gray-50'
                                    }`}
                                >
                                    {result.type_field === 'topic' && (
                                        <h3 className="overflow-hidden text-ellipsis whitespace-nowrap mb-1 text-lg font-semibold text-gray-800">
                                            {result.title}
                                        </h3>
                                    )}
                                    {result.type_field === 'post' && (
                                        <>
                                            <p
                                                className="text-sm text-gray-600 max-h-24 overflow-hidden text-ellipsis"
                                                dangerouslySetInnerHTML={{
                                                    __html:
                                                        new DOMParser().parseFromString(
                                                            result.cooked || '',
                                                            'text/html'
                                                        ).body.textContent || '',
                                                }}
                                            />
                                        </>
                                    )}
                                </div>
                            ))}
                        </div>
                        <div className="w-3/5 p-5 overflow-y-auto">
                            {selectedResult ? (
                                <div>
                                    <SelectedResult result={selectedResult} />
                                </div>
                            ) : (
                                <div className="flex items-center justify-center h-full text-gray-500">
                                    <p>Select a search result to view its details here.</p>
                                </div>
                            )}
                        </div>
                    </div>
                    <Dialog.Close asChild>
                        <button className="mt-4 px-3 py-2 border border-gray-300 rounded-md cursor-pointer self-end">
                            Close
                        </button>
                    </Dialog.Close>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
};
