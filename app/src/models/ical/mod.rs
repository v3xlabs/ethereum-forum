use chrono::{DateTime, TimeZone, Utc};
use icalendar::{CalendarDateTime, Component, DatePerhapsTime, Event};
use meetings::{try_parse_meeting, Meeting};
use poem_openapi::{Enum, Object};
use rrule::RRuleSet;
use serde::{Deserialize, Serialize};

pub mod meetings;
pub mod rich;

#[derive(Debug, Serialize, Deserialize, Object, Clone)]
pub struct CalendarEvent {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub uid: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub start: Option<DateTime<Utc>>,
    pub occurance: EventOccurrence,
    pub meetings: Vec<Meeting>,
    // pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Enum, Clone)]
pub enum EventOccurrence {
    Single,
    Recurring,
}

impl CalendarEvent {
    pub fn from_event(event: Event) -> Result<Vec<Self>, anyhow::Error> {
        let x = event.to_string();
        let mut events = vec![];
        let mut body: String = event.get_description().unwrap_or_default().to_string();
        let meetings: Vec<Meeting> = match try_parse_meeting(&event, &body) {
            Ok((new_body, meetings)) => {
                body = new_body;
                meetings
            }
            Err(_) => vec![],
        };

        if x.contains("RRULE") {
            // Filter out DTSTART, RRULE, RDATE, EXDATE, EXRULE
            let raw_ruleset = x
                .lines()
                .filter(|line| {
                    line.starts_with("DTSTART")
                        || line.starts_with("RRULE")
                        || line.starts_with("RDATE")
                        || line.starts_with("EXDATE")
                        || line.starts_with("EXRULE")
                })
                .collect::<Vec<_>>();

            let ruleset: RRuleSet = raw_ruleset.join("\n").parse()?;
            let rendered_events = ruleset.all(100);
            for start in rendered_events.dates {
                // println!("{:?}", event);
                let start = start.with_timezone(&Utc);

                events.push(CalendarEvent {
                    summary: event.get_summary().map(String::from),
                    description: Some(body.clone()),
                    uid: event.get_uid().map(String::from),
                    last_modified: event.get_last_modified(),
                    created: event.get_created(),
                    start: Some(start),
                    // end,
                    occurance: EventOccurrence::Recurring,
                    meetings: meetings.clone(),
                });
            }
        } else {
            let start = event.get_start().and_then(date_perhaps_time_to_datetime);
            // let end = event.get_end().and_then(date_perhaps_time_to_datetime);
            events.push(CalendarEvent {
                summary: event.get_summary().map(String::from),
                description: Some(body.clone()),
                uid: event.get_uid().map(String::from),
                last_modified: event.get_last_modified(),
                created: event.get_created(),
                start,
                occurance: EventOccurrence::Single,
                meetings,
            });
        }

        Ok(events)
    }
}

fn date_perhaps_time_to_datetime(date_perhaps_time: DatePerhapsTime) -> Option<DateTime<Utc>> {
    match date_perhaps_time {
        DatePerhapsTime::DateTime(calendar_dt) => match calendar_dt {
            CalendarDateTime::Floating(naive_dt) => Some(Utc.from_utc_datetime(&naive_dt)),
            CalendarDateTime::Utc(dt) => Some(dt.into()),
            CalendarDateTime::WithTimezone {
                date_time: naive_dt,
                tzid: _,
            } => Some(Utc.from_utc_datetime(&naive_dt)),
        },
        DatePerhapsTime::Date(naive_date) => {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
            Some(Utc.from_utc_datetime(&naive_dt))
        }
    }
}
