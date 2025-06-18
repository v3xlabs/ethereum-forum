use tracing::{error, info};

use crate::{models::github::GitHubIssue, state::AppState};

pub struct GithubService;

impl GithubService {
    pub async fn start_all_indexers(&self, state: AppState) {
        self.index_repository_issues(&state, "ethereum", "pm").await;
    }

    pub async fn index_repository_issues(&self, state: &AppState, owner: &str, repo: &str) {
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
                        let github_issue = GitHubIssue::from_octocrab(&repository_url, &issue);

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
    }
}
