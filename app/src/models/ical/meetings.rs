use anyhow::Result;
use icalendar::{Event, EventLike};
use poem_openapi::{Object, Union};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize, Union, Clone, PartialEq, Eq)]
#[oai(discriminator_name = "type")]
pub enum Meeting {
    Zoom(ZoomMeetingData),
    Google(GoogleMeetingData),
    Youtube(YoutubeMeetingData),
}

#[derive(Debug, Serialize, Deserialize, Clone, Object, PartialEq, Eq)]
pub struct ZoomMeetingData {
    pub link: String,
    pub meeting_id: Option<String>,
    pub passcode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Object, PartialEq, Eq)]
pub struct GoogleMeetingData {
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Object, PartialEq, Eq)]
pub struct YoutubeMeetingData {
    pub link: String,
}

pub fn try_parse_meeting(event: &Event, body: &str) -> Result<(String, Vec<Meeting>)> {
    let location = event.get_location();
    let mut meetings = vec![];
    let mut new_body = body.to_string();

    if let Some(location) = location {
        info!("location: {}", location);
        // https://ethereumfoundation.zoom.us/j/87569210985?pwd=3Cv1hDh7If4cq9IMXvNln1CtqQ72MR.1
        let eth_zoom_regex =
            Regex::new(r"https://ethereumfoundation.zoom.us/j/(\d+)\?pwd=(\w+)").unwrap();

        if let Some(captures) = eth_zoom_regex.captures(location) {
            let meeting_id = captures[1].to_string();
            let passcode = captures[2].to_string();

            meetings.push(Meeting::Zoom(ZoomMeetingData {
                link: location.to_string(),
                meeting_id: Some(meeting_id),
                passcode: Some(passcode),
            }));
        }

        let youtube_regex = Regex::new(r#"(https://www.youtube.com/live/[^"]+)"#).unwrap();
        if let Some(captures) = youtube_regex.captures(location) {
            let link = captures[1].to_string();
            meetings.push(Meeting::Youtube(YoutubeMeetingData { link }));
        }

        let google_regex = Regex::new(r"https://meet.google.com/[a-zA-Z0-9_\-]+").unwrap();
        if let Some(captures) = google_regex.captures(location) {
            let link = captures[0].to_string();
            meetings.push(Meeting::Google(GoogleMeetingData { link }));
        }
    }

    // 'Join Zoom Meeting'
    // 'is inviting you to a scheduled Zoom meeting.'
    // 'is inviting you to a scheduled Zoom meeting.'

    let x = new_body.split_once("Join Zoom Meeting");
    if let Some((split_body, body)) = x {
        // println!("left todo: {}", body);

        // match the first url in the body
        let link = regex::Regex::new(r#"href="([^"]+?)""#)
            .unwrap()
            .captures(body)
            .map(|m| m[1].to_string())
            .or_else(|| {
                regex::Regex::new(r"https?://[^\s<]+")
                    .unwrap()
                    .captures(body)
                    .map(|m| {
                        let url = m[0].to_string();
                        if url.ends_with("<br/>") {
                            url[..url.len() - 5].to_string()
                        } else {
                            url
                        }
                    })
            });

        let zoom_link_regex =
            Regex::new(r"https://ethereumfoundation.zoom.us/j/(\d+)\?pwd=([\w\.]+)").unwrap();

        let (meeting_id, passcode) = zoom_link_regex
            .captures(link.as_deref().unwrap_or_default())
            .map_or((None, None), |captures| {
                (Some(captures[1].to_string()), Some(captures[2].to_string()))
            });

        new_body = split_body.to_string();
        meetings.push(Meeting::Zoom(ZoomMeetingData {
            link: link.unwrap(),
            meeting_id,
            passcode,
        }));
    }

    let x = new_body.split_once("Zoom Link");
    if let Some((split_body, body)) = x {
        // println!("left todo: {}", body);

        // match the first url in the body
        let link = regex::Regex::new(r#"href="([^"]+?)""#)
            .unwrap()
            .captures(body)
            .map(|m| m[1].to_string());
        let link = link.or_else(|| {
            regex::Regex::new(r"https?://[^\s<]+")
                .unwrap()
                .captures(body)
                .map(|m| {
                    let url = m[0].to_string();
                    if url.ends_with("<br/>") {
                        url[..url.len() - 5].to_string()
                    } else {
                        url
                    }
                })
        });

        let zoom_link_regex =
            Regex::new(r"https://ethereumfoundation.zoom.us/j/(\d+)\?pwd=([\w\.]+)").unwrap();

        let (meeting_id, passcode) = zoom_link_regex
            .captures(link.as_deref().unwrap_or_default())
            .map_or((None, None), |captures| {
                (Some(captures[1].to_string()), Some(captures[2].to_string()))
            });

        new_body = split_body.to_string();
        meetings.push(Meeting::Zoom(ZoomMeetingData {
            link: link.unwrap(),
            meeting_id,
            passcode,
        }));
    }

    let zoom_regex = Regex::new(r"https://us02web.zoom.us/j/(\d+)\?pwd=(\w+)").unwrap();
    if let Some(captures) = zoom_regex.captures(&new_body) {
        let meeting_id = captures[1].to_string();
        let passcode = captures[2].to_string();
        meetings.push(Meeting::Zoom(ZoomMeetingData {
            link: format!("https://us02web.zoom.us/j/{meeting_id}/?pwd={passcode}"),
            meeting_id: Some(meeting_id),
            passcode: Some(passcode),
        }));
    }

    if new_body.contains("Deelnemen via") {
        warn!("deelnemen via google found {}", new_body);
        let link = regex::Regex::new(r"https://meet.google.com/[a-zA-Z0-9_\-]+").unwrap();
        if let Some(captures) = link.captures(&new_body) {
            let link = captures[0].to_string();
            // TODO: update body to remove the link
            // new_body = new_body.replace(&format!("Google Meet: <a {}</a>", link), "");
            debug!("google meet found {}", link);
            meetings.push(Meeting::Google(GoogleMeetingData { link }));
        } else {
            info!("no google meet found {}", new_body);
        }
    }

    // match, extract, and remove the first occurrence of the string
    // "Google Meet: <a href="https://meet.google.com/odf-tghm-ttu"><u>https://meet.google.com/odf-tghm-ttu</u></a>"
    if new_body.contains("Google Meet:") {
        info!("google meet found {}", new_body);
        let link = regex::Regex::new(r"https://meet.google.com/[a-zA-Z0-9_\-]+?").unwrap();
        if let Some(captures) = link.captures(&new_body) {
            let link = captures[0].to_string();
            // TODO: update body to remove the link
            // new_body = new_body.replace(&format!("Google Meet: <a {}</a>", link), "");
            debug!("google meet found {}", link);
            meetings.push(Meeting::Google(GoogleMeetingData { link }));
        } else {
            info!("no google meet found {}", new_body);
        }
    }

    meetings.dedup_by(|a, b| a == b);

    if meetings.is_empty() {
        Err(anyhow::anyhow!("No meeting found"))
    } else {
        Ok((new_body, meetings))
    }
}
