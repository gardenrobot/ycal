use std::{env, process};
use ycal::*;
use ycal::caldav::*;

use chrono::NaiveDate;
use ycal::caldav::CaldavParams;


fn main() {
    let caldav_params = CaldavParams::new("127.0.0.1:5050", "user", "pass", "cal");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need at least 1 arg.");
        process::exit(1);
    };
    if args[1] == "create" {
        if args.len() < 3 {
            eprintln!("Create needs at least 2 args.");
            process::exit(1);
        }
        let date = NaiveDate::parse_from_str(&args[2], "%Y-%m-%d").expect("Error parsing date");
        process_date(ScheduleParams::new(date), caldav_params);
    }
}
