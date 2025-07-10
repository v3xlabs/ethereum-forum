// Type definitions for the forum API responses
export interface TopicSummary {
    discourse_id: string; // changed from id
    topic_id: number;
    title: string;
    post_count: number; // changed from posts_count
    created_at: string;
    last_post_at?: string; // changed from last_posted_at
    view_count: number; // changed from views
    like_count: number;
    participants?: Array<{ id: number; username: string; avatar_template?: string }>;
}

export interface Post {
    topic_id: number;
    post_id: number;
    post_number: number;
    // raw: string; -- doesn't exist in post data
    created_at?: string;
    name?: string; // doesn't exist but should
    // avatar_template?: string; -- doesn't exist
    // like_count: number; -- doesn't exist
    // reply_count: number; -- doesn't exist
    user_id: number;
    discourse_id: string;
}

export interface SearchEntity {
    entity_type: 'topic' | 'post';
    topic_id: number | null;
    post_id: number | null;
    post_number: number | null;
    user_id: number | null;
    username: string | null;
    title: string | null;
    slug: string | null;
    pm_issue: number | null;
    cooked: string | null;
    entity_id: string;
    discourse_id: string; // added to match api
}

export interface SearchResult {
    topics?: TopicSummary[];
    posts?: Post[];
    hits?: number;
}

export interface UserProfile {
    id: number;
    username: string;
    name?: string;
    avatar_template?: string;
    bio_raw?: string;
    location?: string;
    website_name?: string;
    created_at: string;
    last_posted_at?: string;
    last_seen_at?: string;
    post_count: number;
    topic_count: number;
    likes_given: number;
    likes_received: number;
    trust_level: number;
}

export interface ToolResultDisplayProps {
    toolName: string;
    result: string;
    isExpanded: boolean;
    onExpand?: () => void;
}
