export type Participant = {
    id: number;
    name: string;
    username: string;
    flair_url: string | null;
    flair_name: string | null;
    post_count: number;
    flair_color: string | null;
    trust_level: number;
    flair_bg_color: string | null;
    flair_group_id: string | null;
    avatar_template: string;
    primary_group_name: string | null;
};
