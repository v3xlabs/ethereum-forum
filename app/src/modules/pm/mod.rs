use crate::{
    models::pm::{PMData, PMMeetingData},
    state::AppState,
};
use anyhow::Error;
use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;

#[derive(Debug, Clone, Default)]
pub struct PMModule;

pub struct MeetingDataQuery {
    pub discourse_topic_id: Option<String>,
    pub meeting_id: Option<String>,
    pub date: Option<DateTime<Utc>>,
    pub issue_id: Option<String>,
}

impl PMModule {
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    pub async fn get_pm_data(&self) -> Result<PMData, Error> {
        let url = "https://raw.githubusercontent.com/ethereum/pm/refs/heads/master/.github/ACDbot/meeting_topic_mapping.json";
        let client = ClientBuilder::new().use_rustls_tls().build()?;
        let response = client.get(url).send().await?;
        let body = response.text().await?;
        let pm_data: PMData = serde_json::from_str(&body)?;
        Ok(pm_data)
    }

    pub async fn get_pm_data_from_cache(&self, state: &AppState) -> Result<PMData, Error> {
        let x = match state
            .cache
            .pm_data_cache
            .try_get_with("pm_data".to_string(), Self::get_pm_data(self))
            .await
        {
            Ok(x) => x,
            Err(e) => {
                println!("Error fetching cached pm data: {e}");
                return Err(anyhow::anyhow!("Error fetching cached pm data: {}", e));
            }
        };
        Ok(x)
    }

    pub async fn get_by_issue_id(&self, issue_id: u32) -> Result<PMMeetingData, Error> {
        let pm_data = self.get_pm_data().await?;
        let meeting_data = pm_data.values().find(|meeting| {
            let x = meeting.issue_numbers();
            x.contains(&issue_id)
        });

        meeting_data.map_or_else(
            || {
                Err(anyhow::anyhow!(
                    "No meeting data found for issue id: {}",
                    issue_id
                ))
            },
            |meeting_data| Ok(meeting_data.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_get_pm_data() {
        let pm_data = PMModule::new().get_pm_data().await.unwrap();
        println!("{pm_data:?}");
    }
}
