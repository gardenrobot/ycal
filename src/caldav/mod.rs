use crate::Event;
use chrono::Days;
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
        r#"<?xml version="1.0" encoding="utf-8" ?>\n\
          <C:calendar-query xmlns:D="DAV:"\n\
               xmlns:C="urn:ietf:params:xml:ns:caldav">\n\
           <D:prop>\n\
             <D:getetag/>\n\
             <C:calendar-data>\n\
               <C:comp name="VCALENDAR">\n\
                 <C:prop name="VERSION"/>\n\
                 <C:comp name="VEVENT">\n\
                   <C:prop name="SUMMARY"/>\n\
                   <C:prop name="UID"/>\n\
                   <C:prop name="DTSTART"/>\n\
                   <C:prop name="DTEND"/>\n\
                   <C:prop name="DURATION"/>\n\
                   <C:prop name="RRULE"/>\n\
                   <C:prop name="RDATE"/>\n\
                   <C:prop name="EXRULE"/>\n\
                   <C:prop name="EXDATE"/>\n\
                   <C:prop name="RECURRENCE-ID"/>\n\
                 </C:comp>\n\
                 <C:comp name="VTIMEZONE"/>\n\
               </C:comp>\n\
             </C:calendar-data>\n\
           </D:prop>\n\
           <C:filter>\n\
             <C:comp-filter name="VCALENDAR">\n\
               <C:comp-filter name="VEVENT">\n\
                 <C:time-range start="{}"\n\
                               end="{}"/>\n\
               </C:comp-filter>\n\
             </C:comp-filter>\n\
           </C:filter>\n\
          </C:calendar-query>"#,
        start, end
    );

    client
        .request(Method::from_bytes(b"REPORT").unwrap(), params.cal_url())
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

pub fn list_events_by_date(
    rt: &Runtime,
    client: &Client,
    caldav_params: CaldavParams,
    date: NaiveDate,
) -> Vec<String> {
    let response = run_call(
        rt,
        &client,
        build_list_req(client, &caldav_params, date).unwrap(),
    )
    .unwrap();
    println!("{}", response.status());
    println!("{}", rt.block_on(response.text()).unwrap());
    Vec::<String>::new()
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
            DESCRIPTION:adescription\n\
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

        assert_eq!(request.method().as_str(), "REPORT");
        assert_eq!(request.url().as_str(), "https://example.com/user/cal/");
    }
}
