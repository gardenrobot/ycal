use base64::prelude::{Engine, BASE64_STANDARD};
use caldav::CaldavParams;
use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, NaiveDate, NaiveDateTime, Utc,
};
use chrono_tz::{self, Tz};
use regex::Regex;
use reqwest::{get, Client};
use serde_json::{from_str, Result as SJResult, Value};
use std::{borrow::BorrowMut, error::Error};
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::caldav::build_create_req;

pub mod caldav;

const TIMEZONE: Tz = chrono_tz::US::Eastern;

#[derive(PartialEq, Debug)]
pub struct Event {
    start_datetime: DateTime<Tz>,
    end_datetime: DateTime<Tz>,
    title: String,
    description: String,
    changes: Changes,
    studio: String,
    category: String,
    branch: String,
    uid: Option<String>,
    stamp: Option<DateTime<Utc>>,
}

fn tz_dt_to_ical<'a>(dt: DateTime<Tz>) -> DelayedFormat<StrftimeItems<'a>> {
    utc_dt_to_ical(dt.to_utc())
}
fn utc_dt_to_ical<'a>(dt: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'a>> {
    dt.format("%Y%m%dT%H%M%SZ")
}

impl Event {
    pub fn new(
        start_datetime: DateTime<Tz>,
        end_datetime: DateTime<Tz>,
        title: &str,
        description: &str,
        changes: Changes,
        studio: &str,
        category: &str,
        branch: &str,
        uid: Option<String>,
        stamp: Option<DateTime<Utc>>,
    ) -> Event {
        Event {
            start_datetime,
            end_datetime,
            title: String::from(title),
            description: String::from(description),
            changes,
            studio: String::from(studio),
            category: String::from(category),
            branch: String::from(branch),
            uid: match uid {
                Some(x) => Some(String::from(x)),
                None => None,
            },
            stamp,
        }
    }

    pub fn populate(&mut self) -> () {
        if self.uid == None {
            self.uid = Some(String::from(format!("{}", Uuid::new_v4())));
        }
        if self.stamp == None {
            self.stamp = Some(Utc::now());
        }
    }

    pub fn to_ical_str(&mut self) -> String {
        self.populate();

        let uid = self.uid.as_ref().unwrap();
        let dtstart = tz_dt_to_ical(self.start_datetime);
        let dtend = tz_dt_to_ical(self.end_datetime);
        let dtstamp = utc_dt_to_ical(self.stamp.unwrap());
        let summary = &self.title;
        let description = &self.description;

        format!(
            "BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Ycal//Ycal//EN
BEGIN:VEVENT
UID:{}
DTSTAMP:{}
DTSTART:{}
DTEND:{}
SUMMARY:{}
DESCRIPTION:{}
END:VEVENT
END:VCALENDAR",
            uid, dtstamp, dtstart, dtend, summary, description,
        )
    }
}

#[derive(PartialEq, Debug)]
pub enum Changes {
    Normal,
    Subbed(String),
    Cancelled,
}

/// Makes an http call based on the params given. Returns a jquery string with event information.
async fn fetch_schedule(date: NaiveDate) -> Result<String, Box<dyn Error>> {
    let a = "988"; // Likely the org id for the Y
    let location = "5962"; // Waverly
    let date_timestamp = NaiveDateTime::from(date)
        .and_local_timezone(TIMEZONE)
        .unwrap()
        .timestamp();

    let uri = format!(
        "https://groupexpro.com/schedule/embed/json_schedule.php\
        ?schedule=
        &instructor_id=true\
        &a={}\
        &location={}\
        &start={}\
        &end={}",
        a, location, date_timestamp, date_timestamp
    );
    let body = get(uri).await?.text().await?;
    Ok(body)
}

/// Takes the jQuery structure from the Y's site and returns the json obj with schedule-relevant data.
pub fn parse_jquery(input: &str) -> SJResult<Value> {
    // strip the jQuery1234{...}
    let start_index = input.find('{').unwrap();
    let end_index = input.rfind('}').unwrap();
    let input = &input[start_index..end_index + 1];

    let root: Value = from_str(input)?;
    Ok(root["aaData"].clone())
}

/// Takes the json obj representing one event and returns an Event with that data.
pub fn parse_event(json: &Value) -> Result<Event, Box<dyn Error>> {
    let date = String::from(json[0].as_str().unwrap_or_default());
    let (start_time, end_time) = match json[1].as_str().unwrap().split("-").collect::<Vec<_>>()[..]
    {
        [start, end, ..] => (start, end),
        [..] => {
            panic!("Invalid time range: {}", &json[1])
        }
    };

    let format = "%A, %B %d, %Y %I:%M%p";
    let timezone = TIMEZONE;
    let start_datetime = NaiveDateTime::parse_from_str(&format!("{date} {start_time}"), &format)
        .unwrap()
        .and_local_timezone(timezone)
        .single()
        .unwrap();
    let end_datetime = NaiveDateTime::parse_from_str(&format!("{date} {end_time}"), &format)
        .unwrap()
        .and_local_timezone(timezone)
        .single()
        .unwrap();

    let title = json[2].as_str().unwrap_or_default();
    let changes = detect_changes(&json);
    let studio = json[4].as_str().unwrap_or_default();
    let category = json[5].as_str().unwrap_or_default();
    let branch = json[8].as_str().unwrap_or_default();
    let description = String::from_utf8(
        BASE64_STANDARD
            .decode(json[10].as_str().unwrap_or_default())
            .unwrap(),
    )
    .unwrap();

    let studio = match studio.strip_suffix("&nbsp;") {
        Some(x) => x,
        None => &studio,
    };

    Ok(Event::new(
        start_datetime,
        end_datetime,
        title,
        &description,
        changes,
        studio,
        category,
        branch,
        None,
        None,
    ))
}

/// Return an appropriate Changes value based on the given event json
fn detect_changes(json: &Value) -> Changes {
    let details = json[3].as_str().unwrap();
    if details.contains("CANCELED") || details.contains("CANCELLED") {
        return Changes::Cancelled;
    }

    let re = Regex::new(r"\(sub for (.*)\)").unwrap();
    match re.captures(details) {
        Some(c) => {
            let instructor = &c[1];
            Changes::Subbed(String::from(instructor))
        }
        None => Changes::Normal,
    }
}

/// Take a date, retrieve
pub fn process_date(date: NaiveDate, caldav_params: CaldavParams) -> () {
    let rt = Runtime::new().unwrap();
    let schedule = fetch_schedule(date);
    let schedule = rt.block_on(schedule).unwrap();
    let schedule = parse_jquery(&schedule).unwrap();
    let mut event_list = Vec::<Event>::new();
    for event in schedule.as_array().unwrap() {
        let event = parse_event(event).unwrap();
        println!("Found event");
        event_list.push(event);
    }

    let client = Client::new();
    for mut event in event_list {
        println!("Pushing event");
        let request = build_create_req(&client, &caldav_params, event.borrow_mut())
            .expect("Error building create request");
        let response = caldav::run_call(&rt, &client, request).unwrap();
        let status = response.status();
        println!("Response: {status}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    pub fn create_dt(hours_offset: u32) -> DateTime<Tz> {
        NaiveDate::from_ymd_opt(2000, 2, 5)
            .unwrap()
            .and_hms_opt(5 + hours_offset, 0, 0)
            .unwrap()
            .and_local_timezone(TIMEZONE)
            .unwrap()
    }

    pub fn make_event(populate_static: bool) -> Event {
        let mut event = Event::new(
            create_dt(0),
            create_dt(4),
            "atitle",
            "adescription",
            Changes::Normal,
            "astudio",
            "acategory",
            "abranch",
            None,
            None,
        );
        if populate_static {
            event.uid = Some(String::from("e110d27a-1513-40c1-8e8a-db8f50aac1d2"));
            event.stamp = Some(create_dt(6).to_utc());
        }
        event
    }

    #[test]
    fn find_relevant_json() {
        let input = r#"jQuery({
            "aaData": [
                "inside"
            ]
        })"#;
        let expected: Value = from_str("[\"inside\"]").unwrap();

        assert_eq!(parse_jquery(input).unwrap(), expected,);
    }

    #[test]
    fn create_event_from_json() {
        let event = make_event(false);
        let json: Value = from_str(
            r#"[
            "Saturday, February 5, 2000",
            "5:00am-9:00am",
            "atitle",
            "",
            "astudio&nbsp;",
            "acategory",
            "",
            "",
            "abranch",
            "",
            "YWRlc2NyaXB0aW9u", "", "", "", "", ""
        ]"#,
        )
        .unwrap();

        assert_eq!(parse_event(&json).unwrap(), event);
    }

    #[test]
    #[ignore]
    fn test_fetch_schedule() {
        let expected = fs::read_to_string("tests/schedule_2024_04_03.json").unwrap();
        let rt = Runtime::new().unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 4, 3).unwrap();
        let result = rt.block_on(fetch_schedule(date));
        match result {
            Ok(ref result) => assert_eq!(result, &expected),
            _ => panic!(),
        }
    }

    #[test]
    fn test_detect_changes_subbed() {
        let json: Value = from_str(r#"[
            "", "", "",
            "Yoga<br><strong>New Instructor<span class=\"subbed\" style=\"color: #CC0000;\"><br>(sub for Old Instructor)</span></strong><br />",
            "", "", "", "", "", "", "", "", "", "", "", ""
        ]"#).unwrap();

        assert_eq!(
            detect_changes(&json),
            Changes::Subbed(String::from("Old Instructor")),
        );
    }

    #[test]
    fn test_detect_changes_cancelled() {
        let json: Value = from_str(
            r#"[
            "", "", "",
            "CANCELED: Yoga<br><strong>Instructor</strong><br />",
            "", "", "", "", "", "", "", "", "", "", "", ""
        ]"#,
        )
        .unwrap();

        assert_eq!(detect_changes(&json), Changes::Cancelled,);
    }

    #[test]
    fn test_detect_changes_normal() {
        let json: Value = from_str(
            r#"[
            "", "", "",
            "Yoga<br><strong>Instructor</strong><br />",
            "", "", "", "", "", "", "", "", "", "", "", ""
        ]"#,
        )
        .unwrap();

        assert_eq!(detect_changes(&json), Changes::Normal,);
    }

    #[test]
    fn test_to_ical_str() {
        let mut event = make_event(true);

        let ical_str = event.to_ical_str();
        assert_eq!(
            ical_str,
            "BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Ycal//Ycal//EN
BEGIN:VEVENT
UID:e110d27a-1513-40c1-8e8a-db8f50aac1d2
DTSTAMP:20000205T160000Z
DTSTART:20000205T100000Z
DTEND:20000205T140000Z
SUMMARY:atitle
DESCRIPTION:adescription
END:VEVENT
END:VCALENDAR"
        );
    }
}
