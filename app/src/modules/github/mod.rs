use std::{collections::HashMap, sync::Arc, time::Duration};

use async_std::{
    channel::{Receiver, Sender},
    sync::Mutex,
};
use chrono::{DurationRound, TimeDelta, Utc};
use octocrab::Octocrab;
use tracing::{error, info};

use crate::{
    models::github::{GitHubIssue, GitHubIssueComment},
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct GithubConfig {
    pub owner: String,
    pub repo: String,
    pub scrape_interval: String,
}

#[derive(Debug)]
pub struct GithubIndexRequest {
    pub owner: String,
    pub repo: String,
    pub issue_number: Option<u64>,
}

pub struct GithubService {
    indexers: HashMap<String, Arc<GithubIndexer>>,
}

impl GithubService {
    pub async fn new(gh_key: Option<String>) -> Self {
        let mut indexers = HashMap::new();

        if let Some(key) = gh_key {
            rustls::crypto::ring::default_provider()
                .install_default()
                .expect("Failed to install rustls crypto provider");

            let octocrab = Octocrab::builder()
                .personal_token(key)
                .build()
                .expect("Failed to create Octocrab client");

            octocrab::initialise(octocrab);

            if let Err(e) = Self::validate_pat().await {
                error!("GitHub PAT validation failed: {:?}", e);
                panic!("Invalid GitHub Personal Access Token");
            } else {
                info!("GitHub Personal Access Token validated successfully");
            }
        }

        let repo_key = "https://github.com/ethereum/pm";
        let indexer = Arc::new(GithubIndexer::new(GithubConfig {
            owner: "ethereum".to_string(),
            repo: "pm".to_string(),
            scrape_interval: "30m".to_string(),
        }));

        indexers.insert(repo_key.to_string(), indexer);

        Self { indexers }
    }

    async fn validate_pat() -> Result<(), anyhow::Error> {
        let octocrab = octocrab::instance();
        match octocrab.current().user().await {
            Ok(user) => {
                info!("Authenticated as: {}", user.login);
                Ok(())
            }
            Err(e) => {
                error!("Failed to authenticate with GitHub: {:?}", e);
                Err(anyhow::anyhow!("Invalid GitHub Personal Access Token"))
            }
        }
    }

    pub async fn start_all_indexers(&self, state: AppState) {
        for (repo_key, indexer) in &self.indexers {
            let indexer_clone = Arc::clone(indexer);
            let state_clone = state.clone();
            let repo_key_clone = repo_key.clone();

            async_std::task::spawn(async move {
                indexer_clone.run(state_clone).await;
            });

            info!("Started GitHub indexer for repository: {}", repo_key_clone);
        }
    }

    pub async fn enqueue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: Option<u64>,
    ) -> Result<(), anyhow::Error> {
        let repo_key = format!("{}/{}", owner, repo);
        if let Some(indexer) = self.indexers.get(&repo_key) {
            indexer.enqueue(owner, repo, issue_number).await;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "GitHub repository '{}' not found",
                repo_key
            ))
        }
    }
}

/// Individual indexer for a single GitHub repository
pub struct GithubIndexer {
    config: GithubConfig,
    request_tx: Sender<GithubIndexRequest>,
    request_rx: Receiver<GithubIndexRequest>,
    processing_lock: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl GithubIndexer {
    pub fn new(config: GithubConfig) -> Self {
        let (request_tx, request_rx) = async_std::channel::unbounded();

        Self {
            config,
            request_tx,
            request_rx,
            processing_lock: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    pub async fn run(self: Arc<Self>, state: AppState) {
        let state_clone = state.clone();
        let indexer_clone = Arc::clone(&self);
        async_std::task::spawn(async move {
            indexer_clone.fetch_periodically(&state_clone).await;
        });

        info!(
            "Started GitHub indexer for {}/{}, awaiting requests",
            self.config.owner, self.config.repo
        );

        while let Ok(request) = self.request_rx.recv().await {
            info!("Processing GitHub request: {:?}", request);

            if let Some(issue_number) = request.issue_number {
                if let Err(e) = self
                    .index_issue_comments(&state, &request.owner, &request.repo, issue_number)
                    .await
                {
                    error!("Error indexing issue comments: {:?}", e);
                }
            } else {
                // Index all issues
                if let Err(e) = self
                    .index_repository_issues(&state, &request.owner, &request.repo)
                    .await
                {
                    error!("Error indexing repository issues: {:?}", e);
                }
            }

            let key = if let Some(issue_number) = request.issue_number {
                format!("{}/{}#{}", request.owner, request.repo, issue_number)
            } else {
                format!("{}/{}", request.owner, request.repo)
            };

            self.processing_lock.lock().await.remove(&key);
        }

        error!(
            "GitHub indexer for {}/{} stopped",
            self.config.owner, self.config.repo
        );
    }

    pub async fn enqueue(&self, owner: &str, repo: &str, issue_number: Option<u64>) {
        let key = if let Some(issue_number) = issue_number {
            format!("{}/{}#{}", owner, repo, issue_number)
        } else {
            format!("{}/{}", owner, repo)
        };

        let mut set = self.processing_lock.lock().await;
        if set.insert(key.clone()) {
            let request = GithubIndexRequest {
                owner: owner.to_string(),
                repo: repo.to_string(),
                issue_number,
            };

            if let Err(e) = self.request_tx.send(request).await {
                error!("Failed to enqueue GitHub request: {:?}", e);
            } else {
                info!("Enqueued GitHub request: {}", key);
            }
        } else {
            info!("GitHub request {} is already enqueued, skipping", key);
        }
    }

    async fn fetch_periodically(&self, state: &AppState) {
        loop {
            match self.fetch_latest_issues(state).await {
                Ok(_) => info!(
                    "Successfully fetched latest issues for {}/{}",
                    self.config.owner, self.config.repo
                ),
                Err(e) => error!(
                    "Error fetching latest issues for {}/{}: {:?}",
                    self.config.owner, self.config.repo, e
                ),
            }

            let now = Utc::now();
            let next = now.duration_round_up(TimeDelta::minutes(5)).unwrap();

            info!(
                "Next GitHub fetch for {}/{} at: {:?}",
                self.config.owner, self.config.repo, next
            );

            let duration = next.signed_duration_since(now);
            async_std::task::sleep(Duration::from_secs(duration.num_seconds() as u64)).await;
        }
    }

    async fn fetch_latest_issues(&self, state: &AppState) -> anyhow::Result<()> {
        info!(
            "Fetching latest issues for {}/{}",
            self.config.owner, self.config.repo
        );

        // Actually fetch and process issues directly here instead of just enqueueing
        if let Err(e) = self
            .index_repository_issues(state, &self.config.owner, &self.config.repo)
            .await
        {
            error!(
                "Error indexing repository issues during periodic fetch: {:?}",
                e
            );
            return Err(e);
        }

        Ok(())
    }

    async fn index_repository_issues(
        &self,
        state: &AppState,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<()> {
        let octocrab = octocrab::instance();
        let repository_url = format!("https://github.com/{}/{}", owner, repo);

        let mut page = 1u32;
        let per_page = 100u8;

        loop {
            info!(
                "Fetching GitHub issues page {} for {}/{}",
                page, owner, repo
            );

            match octocrab
                .issues(owner, repo)
                .list()
                .per_page(per_page)
                .page(page)
                .send()
                .await
            {
                Ok(issues_page) => {
                    let issues_count = issues_page.items.len();

                    if issues_count == 0 {
                        info!("No more issues to fetch for {}/{}", owner, repo);
                        break;
                    }

                    for issue in issues_page.items {
                        if issue.pull_request.is_some() {
                            continue;
                        }

                        let github_issue = GitHubIssue::from_octocrab(&repository_url, &issue);

                        let should_update = match GitHubIssue::get_by_number(
                            &repository_url,
                            github_issue.number,
                            state,
                        )
                        .await
                        {
                            Ok(Some(existing)) => github_issue.updated_at > existing.updated_at,
                            Ok(None) => true,
                            Err(e) => {
                                error!(
                                    "Error checking existing issue #{}: {:?}",
                                    github_issue.number, e
                                );
                                true
                            }
                        };

                        match github_issue.upsert(state).await {
                            Ok(_) => {
                                info!(
                                    "Upserted GitHub issue: #{} - {}",
                                    github_issue.number, github_issue.title
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error upserting GitHub issue #{}: {:?}",
                                    github_issue.number, e
                                );
                            }
                        }

                        if should_update {
                            info!(
                                "Enqueuing comment fetching for issue #{}",
                                github_issue.number
                            );
                            self.enqueue(owner, repo, Some(github_issue.number as u64))
                                .await;
                        } else {
                            info!(
                                "GitHub issue #{} is up to date, skipping",
                                github_issue.number
                            );
                        }
                    }

                    info!(
                        "Processed {} GitHub issues from page {} for {}/{}",
                        issues_count, page, owner, repo
                    );

                    if (issues_count as u8) < per_page {
                        break;
                    }

                    page += 1;
                }
                Err(e) => {
                    error!(
                        "Error fetching GitHub issues page {} for {}/{}: {:?}",
                        page, owner, repo, e
                    );
                    break;
                }
            }
        }

        info!("Finished indexing GitHub issues for {}/{}", owner, repo);
        Ok(())
    }

    async fn index_issue_comments(
        &self,
        state: &AppState,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> anyhow::Result<()> {
        let octocrab = octocrab::instance();
        let repository_url = format!("https://github.com/{}/{}", owner, repo);
        let mut page = 1u32;
        let per_page = 100u8;

        info!(
            "Fetching comments for issue #{} in {}/{}",
            issue_number, owner, repo
        );

        let issue_number_as_i32 = issue_number as i32;
        let issue_id = match GitHubIssue::get_id_by_number(
            &repository_url,
            issue_number_as_i32,
            state,
        )
        .await
        {
            Ok(Some(id)) => id,
            Ok(None) => {
                error!(
                    "Issue #{} not found in database for {}/{}, skipping comments",
                    issue_number, owner, repo
                );
                return Ok(());
            }
            Err(e) => {
                error!(
                    "Error getting issue ID for #{} in {}/{}: {:?}",
                    issue_number, owner, repo, e
                );
                return Err(e.into());
            }
        };

        loop {
            match octocrab
                .issues(owner, repo)
                .list_comments(issue_number)
                .per_page(per_page)
                .page(page)
                .send()
                .await
            {
                Ok(comments_page) => {
                    let comments_count = comments_page.items.len();

                    if comments_count == 0 {
                        info!(
                            "No more comments to fetch for issue #{} in {}/{}",
                            issue_number, owner, repo
                        );
                        break;
                    }

                    for comment in comments_page.items {
                        let github_comment =
                            GitHubIssueComment::from_octocrab(&repository_url, &issue_id, &comment);

                        match github_comment.upsert(state).await {
                            Ok(_) => {
                                info!(
                                    "Upserted comment {} for issue #{}",
                                    comment.id, issue_number
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Error upserting comment {} for issue #{}: {:?}",
                                    comment.id, issue_number, e
                                );
                            }
                        }
                    }

                    info!(
                        "Processed {} comments for issue #{} in {}/{}",
                        comments_count, issue_number, owner, repo
                    );

                    if (comments_count as u8) < per_page {
                        break;
                    }

                    page += 1;
                }
                Err(e) => {
                    error!(
                        "Error fetching comments for issue #{} in {}/{}: {:?}",
                        issue_number, owner, repo, e
                    );
                    break;
                }
            }
        }

        Ok(())
    }
}
