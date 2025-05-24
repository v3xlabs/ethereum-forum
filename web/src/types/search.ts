export type ForumSearchDocument = {
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

export type TopicSearchResult = ForumSearchDocument & {
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

export type PostSearchResult = ForumSearchDocument & {
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

export type SearchResult = TopicSearchResult | PostSearchResult;
