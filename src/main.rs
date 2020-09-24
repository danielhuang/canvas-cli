mod api;
mod config;

use crate::api::{CanvasAssignment, CanvasCourse};
use anyhow::{Context, Result};
use backoff::{future::FutureOperation as _, Error, ExponentialBackoff};
use chrono::{DateTime, Local};
use colored::Colorize;
use futures::future::try_join_all;
use lazy_static::lazy_static;
use reqwest::Url;
use serde::de::DeserializeOwned;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::builder().build().unwrap();
}

lazy_static! {
    static ref CONFIG: config::Config = config::read_config();
}

async fn fetch<T: DeserializeOwned>(url: &str) -> Result<T> {
    Ok((|| async {
        Ok(CLIENT
            .get(
                Url::from_str(&CONFIG.canvas_url)
                    .unwrap()
                    .join(url)
                    .unwrap(),
            )
            .header("Authorization", format!("Bearer {}", CONFIG.token))
            .send()
            .await
            .with_context(|| format!("fetch {}", url))?
            .json()
            .await
            .with_context(|| format!("parse {}", url))
            .map_err(Error::Permanent)?)
    })
    .retry(ExponentialBackoff {
        initial_interval: Duration::from_millis(10),
        ..Default::default()
    })
    .await?)
}

fn format_time(time: DateTime<Local>) -> String {
    time.format("%I:%M %P").to_string()
}

fn format_datetime(datetime: DateTime<Local>) -> String {
    let today = Local::now().date();
    let time = format_time(datetime);

    if datetime.date() < today {
        format!("{} days ago", (today - datetime.date()).num_days())
    } else if datetime.date() == today {
        format!("today at {}", time)
    } else if datetime.date() == today.succ() {
        format!("tomorrow at {}", time)
    } else if (datetime.date() - today).num_days() < 7 {
        format!("this {} at {}", datetime.date().format("%A"), time)
    } else {
        format!("on {} at {}", datetime.date().format("%b %d"), time)
    }
}

#[derive(StructOpt, Clone, Copy, Debug)]
struct Opt {
    #[structopt(long)]
    show_completed: bool,
    #[structopt(long)]
    show_past: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt: Opt = StructOpt::from_args();

    let all_courses: Vec<CanvasCourse> = fetch("/api/v1/courses?enrollment_state=active").await?;

    let all_assignments = try_join_all(all_courses.into_iter().map(|x| async move {
        fetch::<Vec<CanvasAssignment>>(&format!(
            "/api/v1/courses/{}/assignments?per_page=1000&include=submission",
            x.id
        ))
        .await
        .map(|c| (x, c))
    }))
    .await?;

    let mut all_assignments: Vec<_> = all_assignments
        .into_iter()
        .flat_map(|(c, a)| a.into_iter().map(move |x| (c.clone(), x)))
        .collect();

    all_assignments.sort_by_key(|x| x.1.due_at);
    all_assignments.reverse();

    let now = Local::now();

    for (course, assignment) in all_assignments {
        if let Some(due) = assignment.due_at {
            if opt.show_completed || assignment.submission.submitted_at.is_none() {
                if let Some(points) = assignment.points_possible {
                    println!(
                        "{}",
                        format!(
                            "Due {} - {}{}",
                            if due < now {
                                format_datetime(due).red().bold()
                            } else {
                                format_datetime(due).green().bold()
                            },
                            course.name,
                            if assignment.submission.submitted_at.is_some() {
                                " (completed)".white()
                            } else {
                                "".white()
                            }
                        )
                        .underline()
                    );
                    println!("  {}", assignment.name.trim());
                    println!("  {} points", points);
                    println!("  {}", assignment.html_url);
                    println!();
                }
            }
        }
    }

    Ok(())
}
