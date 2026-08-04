use poem::IntoEndpoint;
use poem_mcpserver::{
    McpServer, Tools,
    content::{Json, Text},
    streamable_http,
};

use crate::{
    models::{
        discourse::user::{DiscourseUserProfile, DiscourseUserSummaryResponse},
        topics::{post::Post, Topic},
    },
    modules::discourse::{ForumSearchDocument, LResult},
    state::AppState,
};

pub struct ForumTools {
    state: AppState,
}

impl ForumTools {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Create a standardized error response for Meilisearch operations
    fn create_error_document(
        error_message: String,
        discourse_id: Option<String>,
        topic_id: Option<i32>,
        user_id: Option<i32>,
        username: Option<String>,
    ) -> ForumSearchDocument {
        ForumSearchDocument {
            entity_type: "error".to_string(),
            discourse_id,
            topic_id,
            post_id: None,
            post_number: None,
            user_id,
            username,
            title: None,
            slug: None,
            pm_issue: None,
            cooked: Some(error_message),
            entity_id: "error".to_string(),
        }
    }
}

#[Tools]
impl ForumTools {
    /// **Get Topic Summary** - Retrieve a comprehensive AI-generated summary of a specific forum topic.
    ///
    /// **Purpose**: This tool provides condensed, high-level overviews of lengthy discussion topics,
    /// making it easier to understand the main points, consensus, and key arguments without reading
    /// through hundreds of posts.
    ///
    /// **When to use**:
    /// - User asks for a "summary" or "overview" of a specific topic
    /// - User wants to understand the main points of a discussion
    /// - User needs a quick introduction to what a topic is about
    /// - User wants to know the current consensus or conclusion of a discussion
    ///
    /// **Input**:
    /// - discourse_id (required): The discourse instance ID
    /// - topic_id (required): The numeric ID of the topic to summarize
    ///
    /// **Output**: Human-readable summary text highlighting key points, decisions, and outcomes
    ///
    /// **Example usage**:
    /// - "What is topic 1234 about?" → Use this tool with topic_id=1234
    /// - "Can you summarize the discussion on EIP-4844?" → First search for the topic, then summarize it
    async fn get_topic_summary(&self, discourse_id: String, topic_id: i32) -> Text<String> {
        match Topic::get_summary_by_topic_id(&discourse_id, topic_id, &self.state, None).await {
            Ok(summary) => Text(summary.summary_text),
            Err(err) => Text(format!("error: {err}")),
        }
    }

    /// **Get Topic Posts** - Retrieve paginated posts from a specific forum topic in chronological order.
    ///
    /// **Purpose**: This tool fetches the actual discussion posts within a topic, allowing you to
    /// examine detailed conversations, read specific arguments, and understand the chronological
    /// flow of the discussion.
    ///
    /// **When to use**:
    /// - User wants to read the actual posts/comments in a discussion
    /// - User needs to see specific details, code examples, or technical discussions
    /// - User wants to understand the chronological progression of a conversation
    /// - User is looking for specific quotes or statements from particular users
    /// - You need more detailed information after getting a topic summary
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - topic_id (required): The numeric ID of the topic
    /// - page (optional, default=1): Page number for pagination (starts at 1)
    /// - size (optional): Number of posts per page (server default applies if not specified)
    ///
    /// **Output**: Array of post objects containing post content, author info, timestamps, and metadata
    ///
    /// **Example usage**:
    /// - "Show me the posts in topic 1234" → get_posts(topic_id=1234)
    /// - "What did users say about gas optimization in topic 5678?" → get_posts(topic_id=5678)
    /// - "Show me page 2 of the discussion" → get_posts(topic_id=1234, page=2)
    async fn get_posts(
        &self,
        discourse_id: String,
        topic_id: i32,
        page: Option<i32>,
        size: Option<i32>,
    ) -> Json<Vec<Post>> {
        let page = page.unwrap_or(1);
        match Post::find_by_topic_id(&discourse_id, topic_id, page, size, &self.state).await {
            Ok((posts, _)) => Json(posts),
            Err(err) => Json(vec![Post {
                discourse_id,
                post_id: -1,
                topic_id,
                user_id: 0,
                post_number: 0,
                updated_at: None,
                created_at: None,
                cooked: Some(format!("error: {err}")),
                post_url: None,
                extra: None,
            }]),
        }
    }

    /// **Search Forum** - Perform full-text search across all forum content (topics and posts).
    ///
    /// **Purpose**: This is the primary search tool for finding relevant discussions, topics, and
    /// posts based on keywords, concepts, or phrases. It searches through both topic titles and
    /// post content to find the most relevant matches.
    ///
    /// **When to use**:
    /// - User asks about any Ethereum-related topic, EIP, or technical concept
    /// - User wants to find discussions related to specific keywords
    /// - User is looking for past conversations about a particular subject
    /// - You need to find relevant context before answering a question
    /// - User asks "What has been discussed about X?" or "Find posts about Y"
    ///
    /// **Search Strategy Tips**:
    /// - Use specific technical terms (e.g., "EIP-1559", "proof of stake", "gas optimization")
    /// - Include related concepts (e.g., search "rollup" for Layer 2 discussions)
    /// - Try broader terms if specific searches don't yield results
    /// - Use this tool first when users ask about ANY Ethereum topic
    ///
    /// **Parameters**:
    /// - query (required): Search terms or phrases
    /// - limit (optional, default=20): Maximum number of results to return
    /// - offset (optional, default=0): Number of results to skip (for pagination)
    ///
    /// **Output**: Array of forum documents (both topics and posts) ranked by relevance
    ///
    /// **Example usage**:
    /// - "What's been discussed about EIP-4844?" → search_forum(query="EIP-4844")
    /// - "Find discussions about MEV" → search_forum(query="MEV maximum extractable value")
    /// - "Layer 2 scaling solutions" → search_forum(query="layer 2 rollup scaling")
    async fn search_forum(
        &self,
        query: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        let Some(meili) = &self.state.meili else {
            return Json(vec![Self::create_error_document(
                "Meilisearch is not configured".to_string(),
                None,
                None,
                None,
                None,
            )]);
        };

        let forum = meili.index("forum");
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        match forum
            .search()
            .with_query(&query)
            .with_limit(limit)
            .with_offset(offset)
            .execute::<ForumSearchDocument>()
            .await
        {
            Ok(results) => {
                let documents: Vec<ForumSearchDocument> =
                    results.hits.into_iter().map(|hit| hit.result).collect();
                Json(documents)
            }
            Err(err) => Json(vec![Self::create_error_document(
                format!("search error: {err}"),
                None,
                None,
                None,
                None,
            )]),
        }
    }

    /// **Search Topics Only** - Search specifically for topic titles and descriptions.
    ///
    /// **Purpose**: This tool focuses on finding discussion topics (not individual posts),
    /// making it useful for discovering what conversations exist about a subject and getting
    /// topic IDs for further exploration.
    ///
    /// **When to use**:
    /// - User wants to see what topics/discussions exist about a subject
    /// - You need to find topic IDs to use with other tools (get_topic_summary, get_posts)
    /// - User asks "What topics are there about X?" or "What discussions exist about Y?"
    /// - You want to see the high-level landscape of discussions on a subject
    ///
    /// **Parameters**:
    /// - query (required): Search terms for topic titles and descriptions
    /// - limit (optional, default=20): Maximum number of topics to return
    /// - offset (optional, default=0): Number of results to skip
    ///
    /// **Output**: Array of topic documents with titles, descriptions, and metadata
    ///
    /// **Example usage**:
    /// - "What topics exist about consensus?" → search_topics(query="consensus")
    /// - "Find discussions about governance" → search_topics(query="governance voting")
    async fn search_topics(
        &self,
        query: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        let Some(meili) = &self.state.meili else {
            return Json(vec![Self::create_error_document(
                "Meilisearch is not configured".to_string(),
                None,
                None,
                None,
                None,
            )]);
        };

        let forum = meili.index("forum");
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        match forum
            .search()
            .with_query(&query)
            .with_filter("entity_type = topic")
            .with_limit(limit)
            .with_offset(offset)
            .execute::<ForumSearchDocument>()
            .await
        {
            Ok(results) => {
                let documents: Vec<ForumSearchDocument> =
                    results.hits.into_iter().map(|hit| hit.result).collect();
                Json(documents)
            }
            Err(err) => Json(vec![Self::create_error_document(
                format!("search error: {err}"),
                None,
                None,
                None,
                None,
            )]),
        }
    }

    /// **Search Posts Only** - Search specifically within post content across all topics.
    ///
    /// **Purpose**: This tool searches through the actual content of forum posts (not topic titles),
    /// making it ideal for finding specific discussions, technical details, code examples, or quotes
    /// within the conversation content.
    ///
    /// **When to use**:
    /// - User wants to find specific technical discussions or implementation details
    /// - User is looking for code examples or technical explanations
    /// - You need to find specific quotes or statements from posts
    /// - User asks for detailed information that would be in post content, not topic titles
    ///
    /// **Parameters**:
    /// - query (required): Search terms for post content
    /// - limit (optional, default=20): Maximum number of posts to return
    /// - offset (optional, default=0): Number of results to skip
    ///
    /// **Output**: Array of post documents with content, author info, and metadata
    ///
    /// **Example usage**:
    /// - "Find posts about Solidity gas optimization" → search_posts(query="Solidity gas optimization")
    /// - "What did people say about merkle trees?" → search_posts(query="merkle tree")
    async fn search_posts(
        &self,
        query: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        let Some(meili) = &self.state.meili else {
            return Json(vec![Self::create_error_document(
                "Meilisearch is not configured".to_string(),
                None,
                None,
                None,
                None,
            )]);
        };

        let forum = meili.index("forum");
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        match forum
            .search()
            .with_query(&query)
            .with_filter("entity_type = post")
            .with_limit(limit)
            .with_offset(offset)
            .execute::<ForumSearchDocument>()
            .await
        {
            Ok(results) => {
                let documents: Vec<ForumSearchDocument> =
                    results.hits.into_iter().map(|hit| hit.result).collect();
                Json(documents)
            }
            Err(err) => Json(vec![Self::create_error_document(
                format!("search error: {err}"),
                None,
                None,
                None,
                None,
            )]),
        }
    }

    /// **Search Posts in Topic** - Search for posts within a specific topic/discussion.
    ///
    /// **Purpose**: This tool allows focused searching within a single topic's posts, useful when
    /// you know which topic contains the information you're looking for and want to find specific
    /// posts or comments within that conversation.
    ///
    /// **When to use**:
    /// - You've already identified a relevant topic (via search_topics or search_forum)
    /// - User wants to find specific information within a known discussion
    /// - You need to locate particular posts or comments within a topic
    /// - User asks about specific aspects of a topic you've already found
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - topic_id (required): The numeric ID of the topic to search within
    /// - query (required): Search terms for finding posts within the topic
    /// - limit (optional, default=20): Maximum number of posts to return
    /// - offset (optional, default=0): Number of results to skip
    ///
    /// **Output**: Array of post documents from within the specified topic
    ///
    /// **Example usage**:
    /// - After finding topic 1234 about "EIP-1559", search for specific aspects:
    ///   search_posts_in_topic(topic_id=1234, query="gas price prediction")
    /// - Find implementation details within a known discussion:
    ///   search_posts_in_topic(topic_id=5678, query="code implementation")
    async fn search_posts_in_topic(
        &self,
        discourse_id: String,
        topic_id: i32,
        query: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        let Some(meili) = &self.state.meili else {
            return Json(vec![Self::create_error_document(
                "Meilisearch is not configured".to_string(),
                Some(discourse_id),
                Some(topic_id),
                None,
                None,
            )]);
        };

        let forum = meili.index("forum");
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        let filter = format!("entity_type = post AND topic_id = {}", topic_id);

        match forum
            .search()
            .with_query(&query)
            .with_filter(&filter)
            .with_limit(limit)
            .with_offset(offset)
            .execute::<ForumSearchDocument>()
            .await
        {
            Ok(results) => {
                let documents: Vec<ForumSearchDocument> =
                    results.hits.into_iter().map(|hit| hit.result).collect();
                Json(documents)
            }
            Err(err) => Json(vec![Self::create_error_document(
                format!("search error: {err}"),
                Some(discourse_id),
                Some(topic_id),
                None,
                None,
            )]),
        }
    }

    /// **Search by User ID** - Find all forum content (topics and posts) created by a specific user.
    ///
    /// **Purpose**: This tool retrieves content from a specific user, useful for understanding their
    /// contributions, expertise areas, or finding discussions they've participated in.
    ///
    /// **When to use**:
    /// - User asks about a specific person's contributions or posts
    /// - You want to find expertise or authority on a subject by checking user contributions
    /// - User wants to see what a particular forum member has discussed
    /// - You need to attribute information or find the source of specific insights
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - user_id (required): The numeric user ID
    /// - query (optional): Additional search terms to filter the user's content
    /// - limit (optional, default=20): Maximum number of results to return
    /// - offset (optional, default=0): Number of results to skip
    ///
    /// **Output**: Array of documents (topics and posts) created by the specified user
    ///
    /// **Note**: Use username_to_user_id first if you only have a username
    async fn search_by_user(
        &self,
        discourse_id: String,
        user_id: i32,
        query: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        let Some(meili) = &self.state.meili else {
            return Json(vec![Self::create_error_document(
                "Meilisearch is not configured".to_string(),
                Some(discourse_id),
                None,
                Some(user_id),
                None,
            )]);
        };

        let forum = meili.index("forum");
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);

        let filter = format!("user_id = {} AND discourse_id = {}", user_id, discourse_id);

        let result = if let Some(q) = query {
            forum
                .search()
                .with_filter(&filter)
                .with_limit(limit)
                .with_offset(offset)
                .with_query(&q)
                .execute::<ForumSearchDocument>()
                .await
        } else {
            forum
                .search()
                .with_filter(&filter)
                .with_limit(limit)
                .with_offset(offset)
                .execute::<ForumSearchDocument>()
                .await
        };

        match result {
            Ok(results) => {
                let documents: Vec<ForumSearchDocument> =
                    results.hits.into_iter().map(|hit| hit.result).collect();
                Json(documents)
            }
            Err(err) => Json(vec![Self::create_error_document(
                format!("search error: {err}"),
                Some(discourse_id),
                None,
                Some(user_id),
                None,
            )]),
        }
    }

    /// **Get User Profile** - Retrieve detailed profile information for a forum user by username.
    ///
    /// **Purpose**: This tool provides comprehensive user information including bio, badges, stats,
    /// and other profile details from the Discourse forum system.
    ///
    /// **When to use**:
    /// - User asks about a specific forum member's background or credentials
    /// - You need to understand someone's role or expertise in the community
    /// - User wants to know more about an author of posts they're reading
    /// - You want to verify the authority or credibility of information sources
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - username (required): The forum username (not display name)
    ///
    /// **Output**: User profile object with bio, stats, badges, and other profile information
    async fn get_user_profile(
        &self,
        discourse_id: String,
        username: String,
    ) -> Json<Option<DiscourseUserProfile>> {
        match self
            .state
            .discourse
            .fetch_discourse_user_cached(&discourse_id, &username)
            .await
        {
            Ok(LResult::Success(profile)) => Json(Some(profile)),
            Ok(LResult::Failed(_)) | Err(_) => Json(None),
        }
    }

    /// **Get User Summary** - Retrieve summary statistics and activity overview for a forum user.
    ///
    /// **Purpose**: This tool provides quick stats about a user's forum activity, including post
    /// counts, topic counts, reputation metrics, and recent activity patterns.
    ///
    /// **When to use**:
    /// - You need quick stats about a user's forum participation
    /// - User asks about someone's activity level or contribution volume
    /// - You want to gauge how active or established a forum member is
    /// - You need metrics for understanding user engagement
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - username (required): The forum username
    ///
    /// **Output**: User summary object with activity statistics and metrics
    async fn get_user_summary(
        &self,
        discourse_id: String,
        username: String,
    ) -> Json<Option<DiscourseUserSummaryResponse>> {
        match self
            .state
            .discourse
            .fetch_discourse_user_summary_cached(&discourse_id, &username)
            .await
        {
            Ok(LResult::Success(summary)) => Json(Some(summary)),
            Ok(LResult::Failed(_)) | Err(_) => Json(None),
        }
    }

    /// **Convert Username to User ID** - Get the numeric user ID for a given username.
    ///
    /// **Purpose**: This utility tool converts usernames to user IDs, which are required by other
    /// user-related tools like search_by_user.
    ///
    /// **When to use**:
    /// - You have a username but need a user ID for other tools
    /// - User mentions someone by username and you want to search their content
    /// - You need to convert username references to numeric IDs for API calls
    ///
    /// **Parameters**:
    /// - username (required): The forum username to convert
    ///
    /// **Output**: Numeric user ID (returns -1 if username not found)
    ///
    /// **Workflow**: Use this tool first, then use the returned ID with search_by_user
    async fn username_to_user_id(&self, discourse_id: String, username: String) -> Json<i32> {
        match self
            .state
            .discourse
            .fetch_discourse_user_cached(&discourse_id, &username)
            .await
        {
            Ok(LResult::Success(profile)) => Json(profile.user.id),
            Ok(LResult::Failed(_)) | Err(_) => Json(-1),
        }
    }

    /// **Search by Username** - Find all content from a user, identified by username.
    ///
    /// **Purpose**: This is a convenience tool that combines username lookup with content search,
    /// allowing you to find a user's forum contributions using their username directly.
    ///
    /// **When to use**:
    /// - User asks about someone's posts using their username
    /// - You want to find contributions from a specific forum member
    /// - User mentions someone by name and you want to see their discussions
    /// - You need to find expertise or insights from a particular community member
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - username (required): The forum username
    /// - query (optional): Additional search terms to filter the user's content
    /// - limit (optional, default=20): Maximum number of results to return
    /// - offset (optional, default=0): Number of results to skip
    ///
    /// **Output**: Array of documents (topics and posts) created by the specified user
    ///
    /// **Example usage**:
    /// - "What has @vitalik posted about?" → search_by_username(discourse_id="magicians", username="vitalik")
    /// - "Find posts by alice about scaling" → search_by_username(discourse_id="magicians", username="alice", query="scaling")
    async fn search_by_username(
        &self,
        discourse_id: String,
        username: String,
        query: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        // First get the user ID
        let user_id = match self
            .state
            .discourse
            .fetch_discourse_user_cached(&discourse_id, &username)
            .await
        {
            Ok(LResult::Success(profile)) => profile.user.id,
            Ok(LResult::Failed(_)) | Err(_) => {
                return Json(vec![Self::create_error_document(
                    format!("User '{}' not found", username),
                    Some(discourse_id),
                    None,
                    None,
                    None,
                )]);
            }
        };

        // Then search by user ID
        self.search_by_user(discourse_id, user_id, query, limit, offset).await
    }

    /// **Search by Username Mention** - Search for content by username, handling @username and /u/username formats.
    ///
    /// **Purpose**: This tool handles different username mention formats commonly used in discussions,
    /// automatically cleaning up the input and finding the user's content.
    ///
    /// **When to use**:
    /// - User provides a username with @ or /u/ prefix (e.g., "@vitalik", "/u/alice")
    /// - You want to handle username mentions in a user-friendly way
    /// - User references someone using typical forum mention syntax
    ///
    /// **Parameters**:
    /// - discourse_id (required): The discourse instance ID
    /// - username_mention (required): Username with or without @, /u/ prefixes
    /// - query (optional): Additional search terms to filter the user's content
    /// - limit (optional, default=20): Maximum number of results
    /// - offset (optional, default=0): Number of results to skip
    ///
    /// **Output**: Array of documents (topics and posts) created by the specified user
    ///
    /// **Example usage**:
    /// - User says "what did @vitalik say about sharding?" → search_by_username_mention(discourse_id="magicians", username_mention="@vitalik", query="sharding")
    /// - User asks about "/u/alice" → search_by_username_mention(discourse_id="magicians", username_mention="/u/alice")
    async fn search_by_username_mention(
        &self,
        discourse_id: String,
        username_mention: String,
        query: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Json<Vec<ForumSearchDocument>> {
        // Clean up the username - handle @username and /u/username formats
        let clean_username = if username_mention.starts_with('@') {
            username_mention.trim_start_matches('@').to_string()
        } else if username_mention.starts_with("/u/") {
            username_mention.trim_start_matches("/u/").to_string()
        } else {
            username_mention
        };

        self.search_by_username(discourse_id, clean_username, query, limit, offset)
            .await
    }
}

pub fn endpoint(state: AppState) -> impl IntoEndpoint {
    streamable_http::endpoint(move |_| McpServer::new().tools(ForumTools::new(state.clone())))
}
