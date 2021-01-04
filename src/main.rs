mod api;
mod config;

use crate::api::{CanvasAssignment, CanvasCourse};
use crate::config::Exclusion;
use backoff::{future::FutureOperation as _, Error, ExponentialBackoff};
use chrono::{DateTime, Local};
use color_eyre::eyre::WrapErr;
use color_eyre::{eyre::eyre, Result, Section};
use colored::Colorize;
use futures::future::try_join_all;
use lazy_static::lazy_static;
use reqwest::Url;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, str::FromStr};
use std::{fmt::Display, time::Duration};
use structopt::StructOpt;

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::builder().build().unwrap();
}

async fn fetch<T: DeserializeOwned>(config: &config::Config, url: &str) -> Result<T> {
    Ok((|| async {
        Ok(CLIENT
            .get(
                Url::from_str(&config.canvas_url)
                    .unwrap()
                    .join(url)
                    .unwrap(),
            )
            .header("Authorization", format!("Bearer {}", config.token))
            .send()
            .await
            .wrap_err_with(|| eyre!("Unable to fetch {}", url))?
            .error_for_status()
            .wrap_err("Server returned error")
            .suggestion("Make sure your credentials are valid")
            .map_err(Error::Permanent)?
            .json()
            .await
            .wrap_err_with(|| eyre!("Unable to parse {}", url))
            .map_err(Error::Permanent)?)
    })
    .retry(ExponentialBackoff {
        initial_interval: Duration::from_millis(10),
        max_elapsed_time: Some(Duration::from_secs(3)),
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
    } else if (datetime.date() - today).num_days() < 14 {
        format!("next {} at {}", datetime.date().format("%A"), time)
    } else {
        format!("on {} at {}", datetime.date().format("%b %d"), time)
    }
}

#[derive(StructOpt, Clone, Copy, Debug)]
struct Opt {
    #[structopt(long)]
    show_completed: bool,
}

fn should_show(assignment: &CanvasAssignment, due_offset: Option<i64>) -> bool {
    if let Some(due) = &assignment.due_at {
        let today = Local::now().date();
        if let Some(offset) = due_offset {
            if (due.date() - today).num_days() < -offset {
                return false;
            }
        }
    }
    if let Some(submission) = &assignment.submission {
        submission.submitted_at.is_none()
            || (assignment.peer_reviews && submission.discussion_entries.len() < 2)
    } else {
        false
    }
}

fn format_submission(assignment: &CanvasAssignment, points: f64) -> String {
    let types = assignment
        .submission_types
        .iter()
        .map(|x| match x.as_str() {
            "online_text_entry" => "Text entry",
            "online_upload" => "File upload",
            "online_quiz" => "Quiz",
            "discussion_topic" => "Discussion",
            "media_recording" => "Media recording",
            "external_tool" => "External tool",
            _ => "Unknown",
        });
    let types: Vec<_> = types.collect();
    let types = types.join(", ");
    format!("{} - {} points", types, points)
}

fn colorize(i: usize, s: &str) -> impl Display {
    [s.blue(), s.yellow(), s.purple(), s.cyan()]
        .iter()
        .cycle()
        .nth(i)
        .unwrap()
        .to_owned()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();
    let config = &config::read_config().wrap_err("Unable to read configuration file")?;

    let mut all_courses: Vec<CanvasCourse> =
        fetch(&config, "/api/v1/courses?enrollment_state=active").await?;

    all_courses.sort_by_key(|x| x.name.clone());

    let order_map: HashMap<_, _> = all_courses
        .iter()
        .map(|x| x.id)
        .enumerate()
        .map(|(i, x)| (x, i))
        .collect();

    let all_assignments = try_join_all(
        all_courses
            .into_iter()
            .filter(|x| {
                !config.exclude.iter().any(|y| match y {
                    Exclusion::ByClassId { class_id } => class_id == &x.id,
                    _ => false,
                })
            })
            .map(|x| async move {
                fetch::<Vec<CanvasAssignment>>(
                    config,
                    &format!(
                        "/api/v1/courses/{}/assignments?per_page=10000&include=submission",
                        x.id
                    ),
                )
                .await
                .map(|c| (x, c))
            }),
    )
    .await?;

    let mut all_assignments: Vec<_> = all_assignments
        .into_iter()
        .flat_map(|(c, a)| a.into_iter().map(move |x| (c.clone(), x)))
        .filter(|(_, a)| {
            !config.exclude.iter().any(|y| match y {
                Exclusion::ByAssignmentId { assignment_id } => assignment_id == &a.id,
                _ => false,
            })
        })
        .collect();

    all_assignments.sort_by_key(|x| x.1.due_at);
    all_assignments.reverse();

    let now = Local::now();

    let mut next_assignment = None;

    for (course, assignment) in all_assignments {
        if let Some(due) = assignment.due_at {
            if opt.show_completed || should_show(&assignment, config.overdue_offset) {
                if let Some(points) = assignment.points_possible {
                    if let Some(submission) = &assignment.submission {
                        println!(
                            "{}",
                            format!(
                                "Due {} - {}{}",
                                if due < now {
                                    format_datetime(due).red().bold()
                                } else {
                                    format_datetime(due).bold()
                                },
                                colorize(order_map[&course.id], &course.name),
                                if submission.submitted_at.is_some() {
                                    " (completed)".white()
                                } else {
                                    "".white()
                                }
                            )
                            .underline()
                        );
                        println!(
                            "  {} {}",
                            assignment.name.trim(),
                            format!("({})", format_submission(&assignment, points)).bright_black()
                        );
                        println!("  {}", assignment.html_url);
                        println!();
                        if due > now && submission.submitted_at.is_none() {
                            next_assignment = Some(assignment);
                        }
                    }
                }
            }
        }
    }

    if let Some(next_assignment) = next_assignment {
        println!(
            "Next assignment is due in {} hours",
            (next_assignment.due_at.unwrap() - now).num_hours()
        )
    }

    Ok(())
}
