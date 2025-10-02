use std::{env, process};
use ycal::{caldav::list_events_by_date, process_date};

use chrono::NaiveDate;
use confy;
use reqwest::Client;
use tokio::runtime::Runtime;

fn main() {
    let caldav_params = confy::load_path("config").expect("Error reading config");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need at least 1 arg.");
        process::exit(1);
    };
    if args[1] == "process" {
        if args.len() < 3 {
            eprintln!("Create needs at least 2 args.");
            process::exit(1);
        }
        let date = NaiveDate::parse_from_str(&args[2], "%Y-%m-%d").expect("Error parsing date");
        println!("Processing {}", date);
        process_date(date, &caldav_params);
    } else if args[1] == "list" {
        if args.len() < 3 {
            eprintln!("Create needs at least 2 args.");
            process::exit(1);
        }
        let rt = Runtime::new().unwrap();
        let client = Client::new();
        let date = NaiveDate::parse_from_str(&args[2], "%Y-%m-%d").expect("Error parsing date");
        list_events_by_date(&rt, &client, &caldav_params, date);
    }
}
