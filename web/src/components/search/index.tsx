import * as Dialog from '@radix-ui/react-dialog';
import { MagnifyingGlassIcon } from '@radix-ui/react-icons';
import { Link } from '@tanstack/react-router';
import { MeiliSearch } from 'meilisearch';
import { useEffect, useState } from 'react';

import { usePost, useTopic } from '@/api/topics';
import { SearchResult, TopicSearchResult } from '@/types/search';

import { TopicPost } from '../topic/TopicPost';
import { TopicPreview } from '../topic/TopicPreview';

if (!import.meta.env.VITE_MEILI_HOST || !import.meta.env.VITE_MEILI_API_KEY) {
    console.warn(
        'MeiliSearch env vars (VITE_MEILI_HOST, VITE_MEILI_API_KEY) are not set. Defaulting to localhost with masterKey'
    );
}

const client = new MeiliSearch({
    host: import.meta.env.VITE_MEILI_HOST || 'http://localhost:7700',
    apiKey: import.meta.env.VITE_MEILI_API_KEY || 'masterKey',
});

const MEILISEARCH_INDEX_NAME = 'forum';

const SelectedPostResult = ({ result, onClose }: { result: SearchResult; onClose: () => void }) => {
    const { data: post, isLoading } = usePost((result.post_id || 0).toString());

    if (isLoading) {
        return <p>Loading post...</p>;
    }

    if (!post || !post.post) {
        return <p>Post not found</p>;
    }

    return (
        <div>
            <Link to={'/t/' + result.topic_id + '#p-' + result.post_id} onClick={onClose}>
                View context
            </Link>
            {post && post.post && <TopicPost post={post!.post} />}
        </div>
    );
};

const SelectedTopicResult = ({
    result,
    onClose,
}: {
    result: TopicSearchResult;
    onClose: () => void;
}) => {
    const { data: topic, isLoading } = useTopic(result.topic_id.toString());

    if (isLoading) {
        return <p>Loading topic...</p>;
    }

    if (!topic) {
        return <p>Topic not found</p>;
    }

    return (
        <div onClick={onClose}>
            <TopicPreview topic={topic} />
        </div>
    );
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
                <Dialog.Content className="fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 bg-primary rounded-lg shadow-lg p-5 w-[90vw] max-w-6xl h-[80vh] max-h-[800px] flex flex-col z-30">
                    <Dialog.Title className="text-xl font-bold mb-4">Search Forum</Dialog.Title>
                    <input
                        type="text"
                        placeholder="Search..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        className="px-3 py-2 mb-4 border border-primary rounded-md text-base w-full"
                    />
                    <div className="flex md:flex-row flex-col flex-grow overflow-hidden border border-primary rounded-md">
                        <div className="md:w-2/5 md:max-h-fit max-h-52 border-r overflow-y-auto p-3 flex flex-col space-y-2.5">
                            {searchResults.length === 0 && searchTerm.trim() !== '' && (
                                <p className="text-gray-500">No results found.</p>
                            )}
                            {searchResults.map((result) => (
                                <div
                                    key={result.id}
                                    onClick={() => handleResultClick(result)}
                                    className={`p-4 rounded-md cursor-pointer transition-all duration-200 ease-in-out shadow-sm hover:shadow ${
                                        selectedResult?.id === result.id
                                            ? 'bg-secondary border-l-4 border-blue-500'
                                            : 'bg-secondary hover:border-l-4 hover:border-gray-300'
                                    }`}
                                >
                                    {result.type_field === 'topic' && (
                                        <>
                                            <div className="flex items-center mb-1.5">
                                                <span className="bg-blue-600 text-white text-xs px-2 py-0.5 rounded-full">
                                                    Topic
                                                </span>
                                            </div>
                                            <h3 className="overflow-hidden text-ellipsis whitespace-nowrap mb-1.5 font-semibold text-gray-800">
                                                {result.title}
                                            </h3>
                                        </>
                                    )}
                                    {result.type_field === 'post' && (
                                        <>
                                            <div className="flex items-center mb-1.5">
                                                <span className="bg-blue-600 text-white text-xs px-2 py-0.5 rounded-full">
                                                    Post
                                                </span>
                                            </div>
                                            <p
                                                className="text-sm text-gray-600 max-h-20 overflow-hidden text-ellipsis line-clamp-3"
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
                        <div className="md:w-3/5 p-5 overflow-y-auto">
                            {selectedResult ? (
                                <div>
                                    {selectedResult.type_field === 'post' && (
                                        <SelectedPostResult
                                            result={selectedResult}
                                            onClose={() => setIsModalOpen(false)}
                                        />
                                    )}
                                    {selectedResult.type_field === 'topic' && (
                                        <SelectedTopicResult
                                            result={selectedResult}
                                            onClose={() => setIsModalOpen(false)}
                                        />
                                    )}
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
