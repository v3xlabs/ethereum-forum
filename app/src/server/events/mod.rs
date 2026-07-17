use futures::{stream, StreamExt};
use poem::{Result, web::Data};
use poem_openapi::{Object, OpenApi, payload::Json};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::models::ical::rich::RichCalendarEvent;
use crate::models::ical::CalendarEvent;
use crate::server::ApiTags;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct EventsApi;

#[OpenApi]
impl EventsApi {
    /// /events
    ///
    /// List events
    #[oai(path = "/events", method = "get", tag = "ApiTags::Events")]
    async fn list(&self, state: Data<&AppState>) -> Result<Json<Vec<RichCalendarEvent>>> {
        if let Some(ical) = &state.ical {
            let events = ical
                .fetch_upcoming(&state)
                .await
                .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::BAD_GATEWAY))?;
            let events: Vec<CalendarEvent> = events.iter().take(32).cloned().collect();

            // async map
            let x = stream::iter(events)
                .then(|event| async {
                    event.rich(&state).await.map_err(|e| {
                        poem::Error::from_string(e.to_string(), StatusCode::BAD_GATEWAY)
                    })
                })
                .collect::<Vec<Result<RichCalendarEvent>>>()
                .await;

            let x = x.into_iter().collect::<Result<Vec<RichCalendarEvent>>>()?;

            return Ok(Json(x));
        }

        Err(poem::Error::from_status(StatusCode::NOT_IMPLEMENTED))
    }

    /// /events/recent
    ///
    /// List recent events
    #[oai(path = "/events/recent", method = "get", tag = "ApiTags::Events")]
    async fn recent(&self, state: Data<&AppState>) -> Result<Json<Vec<RichCalendarEvent>>> {
        if let Some(ical) = &state.ical {
            let events = ical
                .fetch_recent(&state)
                .await
                .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::BAD_GATEWAY))?;
            let events: Vec<CalendarEvent> = events.iter().take(32).cloned().collect();

            // async map
            let x = stream::iter(events)
                .then(|event| async {
                    event.rich(&state).await.map_err(|e| {
                        poem::Error::from_string(e.to_string(), StatusCode::BAD_GATEWAY)
                    })
                })
                .collect::<Vec<Result<RichCalendarEvent>>>()
                .await;

            let x = x.into_iter().collect::<Result<Vec<RichCalendarEvent>>>()?;

            return Ok(Json(x));
        }

        Err(poem::Error::from_status(StatusCode::NOT_IMPLEMENTED))
    }
}
