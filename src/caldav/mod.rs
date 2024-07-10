use chrono::{DateTime, NaiveDateTime, NaiveDate, Days};
use chrono_tz::{self, Tz};
use reqwest::{Result, Response, Client, Method, Request};
use crate::{Event, TIMEZONE, Changes};
use core::future::Future;
use tokio::runtime::Runtime;

pub struct CaldavParams {
    url: String, // TODO add protocol
    user: String,
    pass: String,
    calendar: String,
}

impl CaldavParams {
    pub fn new(url: &str, user: &str, pass: &str, calendar: &str) -> CaldavParams {
        CaldavParams {
            url: String::from(url),
            user: String::from(user),
            pass: String::from(pass),
            calendar: String::from(calendar),
        }
    }

    pub fn cal_url(&self) -> String {
        let cal_url = format!("http://{}/{}/{}/", self.url, self.user, self.calendar);
        eprintln!("{cal_url}"); // TODO
        return cal_url;
    }

    pub fn event_url(&self, event_uid: &str) -> String {
        let cal_url = format!("http://{}/{}/{}/{}.ics", self.url, self.user, self.calendar, event_uid);
        eprintln!("{cal_url}");
        return cal_url;
    }
}

/// Build a request to create an event.
pub fn build_create_req(client: &Client, params: &CaldavParams, event: &mut Event) -> Result<Request> {
    event.populate();
    client.request(Method::PUT, params.event_url(event.uid.as_ref().unwrap()))
        .body(event.to_ical_str())
        .header("Content-Type", "text/calendar")
        .basic_auth(&params.user, Some(&params.pass))
        .build()
}

pub fn run_call(rt: &Runtime, client: &Client, request: Request) -> Result<Response> {
    rt.block_on(client.execute(request))
}

pub fn build_delete_event(client: &Client, event_id: &str, params: &CaldavParams) -> Result<Request> {
    client.request(Method::DELETE, params.event_url(event_id))
        .basic_auth(&params.user, Some(&params.pass))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::{create_dt, make_event};
    use std::borrow::BorrowMut;
    use std::process;

    #[test]
    fn test_build_create_req() {
        let client = Client::new();
        let mut event = make_event(true);
        let url = "https://example.com";
        let params = CaldavParams::new(url, "user", "pass", "cal");
        let request = build_create_req(&client, &params, event.borrow_mut()).unwrap();

        let body = r#"BEGIN:VCALENDAR
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
END:VCALENDAR"#;
        println!("Expected: {body}");
        println!("Actual: {}", event.to_ical_str());
        assert_eq!(request.body().unwrap().as_bytes().unwrap(), body.as_bytes());
    }
}
