use crate::Event;
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

pub fn run_call(rt: &Runtime, client: &Client, request: Request) -> Result<Response> {
    rt.block_on(client.execute(request))
}

pub fn build_delete_req(client: &Client, params: &CaldavParams, event_id: &str) -> Result<Request> {
    client
        .request(Method::DELETE, params.event_url(event_id))
        .basic_auth(&params.user, Some(&params.pass))
        .build()
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
}
