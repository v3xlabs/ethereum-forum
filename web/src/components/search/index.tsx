import * as Dialog from '@radix-ui/react-dialog';
import { MagnifyingGlassIcon } from '@radix-ui/react-icons';
import { MeiliSearch } from 'meilisearch';
import { useEffect, useState } from 'react';

const client = new MeiliSearch({
    host: 'http://localhost:7700',
    apiKey: 'masterKey',
});

const MEILISEARCH_INDEX_NAME = 'forum';

type TopicResult = {
    id: number;
    title: string;
    slug: string;
    post_number: number;
    user_id: number;
    pm_issue: number;
    type_field: 'topic';
};

type PostResult = {
    id: number;
    title: string;
    slug: string;
    post_number: number;
    user_id: number;
    pm_issue: number;
    type_field: 'post';
    cooked: string;
};

type SearchResult = TopicResult | PostResult;

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
                                        <h3
                                            className="overflow-hidden text-ellipsis whitespace-nowrap mb-1 text-lg font-semibold text-gray-800"
                                            dangerouslySetInnerHTML={{
                                                __html: result.title || 'Untitled Topic',
                                            }}
                                        />
                                    )}
                                    {result.type_field === 'post' && (
                                        <>
                                            <p
                                                className="text-sm text-gray-600 max-h-24 overflow-hidden text-ellipsis"
                                                dangerouslySetInnerHTML={{
                                                    __html: result.cooked || 'No content',
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
                                    <div
                                        className="text-base leading-relaxed prose max-w-none"
                                        dangerouslySetInnerHTML={{
                                            __html:
                                                selectedResult?.type_field === 'post'
                                                    ? selectedResult.cooked
                                                    : selectedResult?.title ||
                                                      'Select an item to see details.',
                                        }}
                                    />
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
