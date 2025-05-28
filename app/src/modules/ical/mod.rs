use anyhow::Error;
use chrono::{Duration, Utc};
use figment::{providers::Env, Figment};
use icalendar::{Calendar, CalendarComponent};
use serde::Deserialize;
use tracing::info;

use crate::{models::ical::CalendarEvent, state::AppState};

#[derive(Debug, Deserialize)]
pub struct ICalConfig {
    pub url: String,
}

pub fn init_ical(figment: Figment) -> Option<ICalConfig> {
    let config = figment
        .merge(Env::prefixed("ICAL_"))
        .extract::<ICalConfig>();

    match config {
        Ok(config) => Some(config),
        Err(e) => {
            info!("No ICal config found: {}", e);
            None
        }
    }
}

impl ICalConfig {
    pub async fn fetch(&self) -> Result<Vec<CalendarEvent>, Error> {
        let response = reqwest::get(&self.url).await?;
        let body = response.text().await?;

        let cal: Calendar = body.parse().unwrap();
        let mut events: Vec<CalendarEvent> = Vec::new();

        // keep 365 days of history
        let now = Utc::now() - Duration::days(365);
        for calendar in cal.components {
            if let CalendarComponent::Event(event) = calendar {
                let parsed_events = match CalendarEvent::from_event(&event) {
                    Ok(events) => events,
                    Err(e) => {
                        println!("Error parsing event: {e}");
                        continue;
                    }
                };

                for event in parsed_events {
                    if let Some(start) = event.start {
                        if start >= now {
                            events.push(event);
                        }
                    }
                }
            }
        }
        events.sort_by_key(|event| event.start.unwrap());
        Ok(events)
    }

    pub async fn fetch_cached(&self, state: &AppState) -> Result<Vec<CalendarEvent>, Error> {
        let x = match state
            .cache
            .ical_cache
            .try_get_with(self.url.clone(), Self::fetch(self))
            .await
        {
            Ok(x) => x,
            Err(e) => {
                println!("Error fetching cached ical: {e}");
                return Err(anyhow::anyhow!("Error fetching cached ical: {}", e));
            }
        };
        Ok(x)
    }

    pub async fn fetch_upcoming(&self, state: &AppState) -> Result<Vec<CalendarEvent>, Error> {
        let events = self.fetch_cached(state).await?;
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let upcoming = events
            .iter()
            .filter(|event| event.start.unwrap() >= now)
            .cloned()
            .collect();
        Ok(upcoming)
    }

    pub async fn fetch_recent(&self, state: &AppState) -> Result<Vec<CalendarEvent>, Error> {
        let events = self.fetch_cached(state).await?;
        let now = Utc::now();
        let recent = events
            .iter()
            .rev()
            .filter(|event| event.start.unwrap() < now)
            .cloned()
            .collect();
        Ok(recent)
    }
}

#[cfg(test)]
mod tests {
    use figment::providers::Env;

    use super::*;

    #[async_std::test]
    async fn test_fetch_ical() {
        dotenvy::dotenv().ok();
        let config = init_ical(Figment::new().merge(Env::prefixed("ICAL_"))).unwrap();
        let events = config.fetch().await.unwrap();

        println!("{events:?}");
    }
}
