use serde::{Deserialize, Serialize};

use crate::models::discourse::user::DiscourseUser;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscourseLatestResponse {
    pub users: Vec<DiscourseUser>,
    // primary_groups: serde_json::Value, // Vec<unknown>
    // flair_groups: serde_json::Value, // Vec<unknown>
    pub topic_list: DiscourseLatestTopicList,
    #[serde(flatten)]
    extra: serde_json::Value, // unknown
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscourseLatestTopicList {
    // can_create_topic: bool,
    pub more_topics_url: Option<String>, // if None, no more topics to fetch
    per_page: u32,
    // top_tags: Vec<String>,
    pub topics: Vec<DiscourseLatestTopic>,
    #[serde(flatten)]
    extra: serde_json::Value, // unknown
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscourseLatestTopic {
    pub id: i32,
    pub title: String,
    pub fancy_title: String,
    pub slug: String,
    pub posts_count: i32,
    pub reply_count: i32,
    pub highest_post_number: u32,
    pub image_url: Option<String>,
    // pub created_at: String,
    // pub last_posted_at: String,
    // pub archetype: String,
    // pub unseen: bool,
    pub pinned: bool,
    pub unpinned: Option<String>, // unknown
    pub visible: bool,
    pub closed: bool,
    pub archived: bool,
    // pub bookmarked: Option<String>, // unknown
    // pub liked: Option<String>, // unknown
    // pub tags: Vec<String>, // Vec<unknown>
    // pub tags_descriptions: serde_json::Value, // unknown
    pub views: u32,
    pub like_count: u32,
    // pub has_summary: bool,
    // pub last_poster_username: String,
    pub category_id: u32,
    // pub pinned_globally: bool,
    pub featured_link: Option<String>, // unknown
    // pub posters: Vec<DiscourseLatestTopicPoster>,
    #[serde(flatten)]
    pub extra: serde_json::Value, // unknown
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscourseLatestTopicPoster {
    extras: Option<String>, // unknown
    description: String,
    // i32 because system user is `-1`
    // user_id: i32,
    // primary_group_id: Option<i32>, // unknown
    // flair_group_id: Option<i32>, // unknown
    #[serde(flatten)]
    extra: serde_json::Value, // unknown
}
