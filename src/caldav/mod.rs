use crate::{Changes, Event};
use chrono::{DateTime, Days};
use chrono::{NaiveDate, NaiveDateTime};
use reqwest::{Client, Method, Request, Response, Result};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Serialize, Deserialize, Debug)]
pub struct CaldavParams {
    protocol: String,
    url: String,
    user: String,
    pass: String,
    calendar: String,
}

impl CaldavParams {
    pub fn new(protocol: &str, url: &str, user: &str, pass: &str, calendar: &str) -> CaldavParams {
        CaldavParams {
            protocol: String::from(protocol),
            url: String::from(url),
            user: String::from(user),
            pass: String::from(pass),
            calendar: String::from(calendar),
        }
    }

    pub fn cal_url(&self) -> String {
        format!(
            "{}://{}/{}/{}/",
            self.protocol, self.url, self.user, self.calendar
        )
    }

    pub fn event_url(&self, event_uid: &str) -> String {
        format!(
            "{}://{}/{}/{}/{}.ics",
            self.protocol, self.url, self.user, self.calendar, event_uid
        )
    }
}

impl ::std::default::Default for CaldavParams {
    fn default() -> Self {
        Self {
            protocol: String::from("https"),
            url: String::from("example.com"),
            user: String::from("user"),
            pass: String::from("pass"),
            calendar: String::from("cal"),
        }
    }
}

/// Build a request to create an event.
pub fn build_create_req(
    client: &Client,
    params: &CaldavParams,
    event: &mut Event,
) -> Result<Request> {
    event.populate();
    client
        .request(Method::PUT, params.event_url(event.uid.as_ref().unwrap()))
        .body(event.to_ical_str())
        .header("Content-Type", "text/calendar")
        .basic_auth(&params.user, Some(&params.pass))
        .build()
}

pub fn build_list_req(
    client: &Client,
    params: &CaldavParams,
    date: NaiveDate,
) -> Result<Request> {
    let start = NaiveDateTime::from(date).and_utc().format("%Y%m%dT%H%M%SZ");
    let end = NaiveDateTime::from(date.checked_add_days(Days::new(1)).unwrap())
        .and_utc()
        .format("%Y%m%dT%H%M%SZ");
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8" ?>
          <C:calendar-query xmlns:D="DAV:"
               xmlns:C="urn:ietf:params:xml:ns:caldav">
           <D:prop>
             <D:getetag/>
             <C:calendar-data>
               <C:comp name="VCALENDAR">
                 <C:prop name="VERSION"/>
                 <C:comp name="VEVENT">
                   <C:prop name="SUMMARY"/>
                   <C:prop name="UID"/>
                   <C:prop name="DTSTART"/>
                   <C:prop name="DTEND"/>
                   <C:prop name="DURATION"/>
                   <C:prop name="RRULE"/>
                   <C:prop name="RDATE"/>
                   <C:prop name="EXRULE"/>
                   <C:prop name="EXDATE"/>
                   <C:prop name="RECURRENCE-ID"/>
                 </C:comp>
                 <C:comp name="VTIMEZONE"/>
               </C:comp>
             </C:calendar-data>
           </D:prop>
           <C:filter>
             <C:comp-filter name="VCALENDAR">
               <C:comp-filter name="VEVENT">
                 <C:time-range start="{}"
                               end="{}"/>
               </C:comp-filter>
             </C:comp-filter>
           </C:filter>
          </C:calendar-query>"#,
        start, end
    );

    client
        .request(Method::from_bytes(b"GET").unwrap(), params.cal_url())
        .body(body)
        .header("Content-Type", "text/xml")
        .header("Depth", "1")
        .basic_auth(&params.user, Some(&params.pass))
        .build()
}

pub fn run_call(rt: &Runtime, client: &Client, request: Request) -> Result<Response> {
    rt.block_on(client.execute(request))
}

pub fn build_delete_req(client: &Client, params: &CaldavParams, event_id: &str) -> Result<Request> {
    client
        .request(Method::DELETE, params.event_url(event_id))
        .basic_auth(&params.user, Some(&params.pass))
        .build()
}

use serde_xml_rs;


#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct XMLMultiStatus {
    response: Option<Vec<XMLResponse>>
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct XMLResponse {
    href: String,
    propstat: XMLPropstat,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct XMLPropstat {
    prop: XMLProp,
    status: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct XMLProp {
    getetag: String,
    #[serde(rename = "calendar-data")]
    calendarData: String,
}

/**
 * Takes an ical string and parses it to Events.
 */
fn parse_event_list(
    event_list_str: &str
) -> Vec<Event> {

    let mut event_list= Vec::new();

    let event_list_str = event_list_str.split_inclusive("BEGIN:VEVENT");
    
    for event_str in event_list_str {
        if event_str.contains("END:VEVENT") {
            let event = parse_event(event_str);
            event_list.push(event);
        }
    }
    event_list
}

use ical;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::io::BufReader;

use std::fs::File;

fn parse_event(
    event: &str
) -> Event {
    /* Parse an ical event string into an Event.
     */
    let properties: ical::PropertyParser<&[u8]> = ical::PropertyParser::from_reader(event.as_bytes());

    // TODO have better checking here.
    let mut start_datetime = None;
    let mut end_datetime = None;
    let mut title = Some(String::new());
    let mut description = Some(String::new());
    let mut changes = Some(Changes::Normal);
    let mut studio = Some("TODO".to_string());
    let mut category = Some("TODO".to_string());
    let mut branch = Some("TODO".to_string());
    let mut uid = None;
    let mut stamp = None;

    for property in properties {
        let property = property.unwrap();
        let mut property_value = property.value.unwrap();

        match property.name.as_str() {
            "DTSTART" => start_datetime = {
                Some(NaiveDateTime::parse_from_str(&property_value, "%Y%m%dT%H%M%SZ").unwrap().and_local_timezone(super::TIMEZONE).single().unwrap())
            },
            "DTEND" => end_datetime = {
                Some(NaiveDateTime::parse_from_str(&property_value, "%Y%m%dT%H%M%SZ").unwrap().and_local_timezone(super::TIMEZONE).single().unwrap())
            },
            "SUMMARY" => title = Some(property_value.clone()),
            "UID" => uid = Some(property_value.clone()),
            "DTSTAMP" => stamp = {
                Some(NaiveDateTime::parse_from_str(&property_value, "%Y%m%dT%H%M%SZ").unwrap().and_utc())
            },
            "LOCATION" => {
                let split = property_value.split_at(property_value.find("\\, ").unwrap());
                studio = Some(String::from(split.0));
                branch = Some(String::from(&split.1[3..]));
            },
            "DESCRIPTION" => {
                let split = property_value.split_at(property_value.find("\\n").unwrap());
                category = Some(String::from(split.0));
                description = Some(String::from(&split.1[2..]));
            },
            _ => (),
        }
    }

    Event::new(
        start_datetime.unwrap(),
        end_datetime.unwrap(),
        &title.unwrap(),
        &description.unwrap(),
        changes.unwrap(),
        &studio.unwrap(),
        &category.unwrap(),
        &branch.unwrap(),
        Some(uid.unwrap()),
        Some(stamp.unwrap()),
    )
}

pub fn list_events_by_date(
    rt: &Runtime,
    client: &Client,
    caldav_params: &CaldavParams,
    date: NaiveDate,
) -> Vec<Event> {
    let req = build_list_req(client, &caldav_params, date).unwrap();
    let response = run_call(
        rt,
        &client,
        req,
    )
    .unwrap();
    let text = rt.block_on(response.text()).unwrap();

    let events = parse_event_list(&text);

    events
}

pub fn escape_ical_value(value: &str) -> String {
    let mut escaped = String::from("");
    for (index, character) in value.chars().enumerate() {
        let escape_map: HashMap<char, &str> = [
            (',', "\\,"),
            (';', "\\;"),
            ('\\', "\\\\"),
            ('\n', "\\n"),
        ].iter().cloned().collect();
        if escape_map.contains_key(&character) {
            escaped.push_str(escape_map[&character]);
        } else {
            escaped.push(character);
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::super::tests::make_event;
    use super::*;
    use std::borrow::BorrowMut;

    #[test]
    fn test_build_create_req() {
        let client = Client::new();
        let mut event = make_event(true);
        let protocol = "https";
        let url = "example.com";
        let params = CaldavParams::new(protocol, url, "user", "pass", "cal");
        let request = build_create_req(&client, &params, event.borrow_mut()).unwrap();

        let body = 
            "BEGIN:VCALENDAR\n\
            VERSION:2.0\n\
            PRODID:-//Ycal//Ycal//EN\n\
            BEGIN:VEVENT\n\
            UID:e110d27a-1513-40c1-8e8a-db8f50aac1d2\n\
            DTSTAMP:20000205T160000Z\n\
            DTSTART:20000205T100000Z\n\
            DTEND:20000205T140000Z\n\
            SUMMARY:atitle\n\
            DESCRIPTION:acategory\\nadescription\n\
            LOCATION:astudio\\, abranch\n\
            END:VEVENT\n\
            END:VCALENDAR";
        assert_eq!(request.method(), Method::PUT);
        assert_eq!(
            request.url().as_str(),
            "https://example.com/user/cal/e110d27a-1513-40c1-8e8a-db8f50aac1d2.ics"
        );
        assert_eq!(request.body().unwrap().as_bytes().unwrap(), body.as_bytes());
    }

    #[test]
    fn test_build_delete_req() {
        let client = Client::new();
        let event = make_event(true);
        let protocol = "https";
        let url = "example.com";
        let params = CaldavParams::new(protocol, url, "user", "pass", "cal");
        let request = build_delete_req(&client, &params, &event.uid.unwrap()).unwrap();

        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.url().as_str(),
            "https://example.com/user/cal/e110d27a-1513-40c1-8e8a-db8f50aac1d2.ics"
        );
        assert!(request.body().is_none());
    }

    #[test]
    fn test_build_list_req() {
        let client = Client::new();
        let protocol = "https";
        let url = "example.com";
        let params = CaldavParams::new(protocol, url, "user", "pass", "cal");
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let request = build_list_req(&client, &params, date).unwrap();

        assert_eq!(request.method().as_str(), "GET");
        assert_eq!(request.url().as_str(), "https://example.com/user/cal/");
    }

    #[test]
    #[ignore]
    fn test_list_events_by_date() {
        let client = Client::new();
        let caldav_params = CaldavParams::new("http", "127.0.0.1:5232", "user", "pass", "cal");
        let rt = Runtime::new().unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
        let event_list = list_events_by_date(&rt, &client, &caldav_params, date);
        let mut expected_event_list = Vec::new(); // TODO
        assert_eq!(event_list, expected_event_list);
    }

    #[test]
    fn test_parse_event_list() {
        let mut expected_event_list = Vec::new();
        expected_event_list.push(make_event(true));
        let event_list_str = "BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Ycal//Ycal//EN
BEGIN:VEVENT
UID:e110d27a-1513-40c1-8e8a-db8f50aac1d2
DTSTART:20000205T050000Z
DTEND:20000205T090000Z
DESCRIPTION:acategory\\nadescription
DTSTAMP:20000205T160000Z
LOCATION:astudio\\, abranch
SUMMARY:atitle
END:VEVENT
END:VCALENDAR";

        let event_list = parse_event_list(event_list_str);
        assert_eq!(event_list, expected_event_list);
    }

    #[test]
    fn test_escape_ical_value() {
        let value = "New York City, NY; USA\\ \nnewline";
        assert_eq!(
            escape_ical_value(value),
            "New York City\\, NY\\; USA\\\\ \\nnewline"
        );
    }
}

