mod api;
mod config;
mod progress;

use crate::api::{CanvasAssignment, CanvasCourse};
use crate::config::Exclusion;
use backoff::{future::FutureOperation as _, Error, ExponentialBackoff};
use chrono::{DateTime, Local};
use color_eyre::eyre::WrapErr;
use color_eyre::{eyre::eyre, Result, Section};
use colored::Colorize;
use config::{config_path, Inclusion};
use futures::future::try_join_all;
use lazy_static::lazy_static;
use progress::Progress;
use reqwest::Url;
use serde::de::DeserializeOwned;
use std::time::Duration;
use std::{
    cmp::{max, min},
    collections::HashMap,
    str::FromStr,
};
use structopt::StructOpt;
use tokio::{
    fs::{read_to_string, File},
    io::AsyncWriteExt,
};
use toml_edit::{value, ArrayOfTables, Document, Table};

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::builder().build().unwrap();
}

fn decode_json<T: DeserializeOwned>(x: &[u8]) -> Result<T> {
    let jd = &mut serde_json::Deserializer::from_slice(x);

    Ok(serde_path_to_error::deserialize(jd)?)
}

async fn fetch<T: DeserializeOwned>(config: &config::Config, url: &str) -> Result<T> {
    Ok((|| async {
        decode_json(
            &CLIENT
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
                .bytes()
                .await
                .wrap_err("Failed to read data from server")
                .map_err(Error::Permanent)?,
        )
        .wrap_err_with(|| eyre!("Unable to parse {}", url))
        .map_err(Error::Permanent)
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

    if datetime.date() == today {
        format!("today at {}", time)
    } else if datetime.date() == today.succ() {
        format!("tomorrow at {}", time)
    } else if (0..7).contains(&(datetime.date() - today).num_days()) {
        format!("this {} at {}", datetime.date().format("%A"), time)
    } else if (7..14).contains(&(datetime.date() - today).num_days()) {
        format!("next {} at {}", datetime.date().format("%A"), time)
    } else {
        format!("on {} at {}", datetime.date().format("%b %d"), time)
    }
}

fn format_duration(a: DateTime<Local>, b: DateTime<Local>) -> String {
    assert!(b > a);
    if (b - a).num_hours() == 1 {
        "1 hour".into()
    } else if b - a < chrono::Duration::hours(48) {
        format!("{} hours", (b - a).num_hours())
    } else {
        format!("{} days", (b.date() - a.date()).num_days())
    }
}

fn format_duration_full(a: DateTime<Local>, b: DateTime<Local>) -> String {
    let base_text = format_duration(min(a, b), max(a, b));
    if b > a {
        format!("in {}", base_text)
    } else {
        format!("{} ago", base_text)
    }
}

#[derive(StructOpt, Clone, Debug)]
enum Opt {
    #[structopt(about = "Displays a list of upcoming assignments")]
    Todo {
        #[structopt(long)]
        show_all: bool,
    },
    #[structopt(about = "Adds an assignment to the exclusion list")]
    Exclude { assignment_id: i64 },
}

fn should_show(config: &config::Config, assignment: &CanvasAssignment) -> bool {
    if config.include.contains(&Inclusion::ByAssignmentId {
        assignment_id: assignment.id,
    }) {
        return true;
    }

    if let Some(due) = assignment.due_at {
        if let Some(overdue_offset) = config.hide_overdue_after_days {
            if (Local::now() - due).num_days() > overdue_offset {
                return false;
            }
        }
    }

    if let Some(submission) = &assignment.submission {
        if !(submission.submitted_at.is_none()
            || assignment.peer_reviews && submission.discussion_entries.len() < 2)
        {
            return false;
        }
    }

    true
}

fn process_submission(assignment: &CanvasAssignment, points: f64) -> (String, bool) {
    let mut online_submission = false;
    let types = assignment
        .submission_types
        .iter()
        .map(|x| match x.as_str() {
            "none" => "No submission".to_string(),
            "on_paper" => "On paper".to_string(),
            x => {
                let text = match x {
                    "online_text_entry" => "Text entry",
                    "online_upload" => "File upload",
                    "online_quiz" => "Quiz",
                    "discussion_topic" => "Discussion",
                    "media_recording" => "Media recording",
                    "external_tool" => "External tool",
                    _ => "Unknown",
                };
                online_submission = true;
                text.purple().to_string()
            }
        });
    let types: Vec<_> = types.collect();
    let types = types.join(", ");
    (format!("{} - {} points", types, points), online_submission)
}

fn colorize(i: usize, s: &str) -> String {
    [s.blue(), s.yellow(), s.purple(), s.cyan()]
        .iter()
        .cycle()
        .nth(i)
        .unwrap()
        .to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();
    let config = &config::read_config().wrap_err("Unable to read configuration file")?;

    match opt {
        Opt::Todo { show_all } => {
            run_todo(config, show_all).await?;
        }
        Opt::Exclude { assignment_id } => {
            run_exclude(assignment_id).await?;
        }
    }

    Ok(())
}

async fn run_exclude(assignment_id: i64) -> Result<()> {
    let old_config = read_to_string(config_path()).await?;
    let mut doc: Document = old_config.parse()?;

    doc["exclude"]
        .as_array_of_tables_mut()
        .unwrap_or(&mut ArrayOfTables::default())
        .append({
            let mut t = Table::new();
            t["assignment_id"] = value(assignment_id);
            t
        });

    File::create(config_path())
        .await?
        .write_all(doc.to_string().as_bytes())
        .await?;

    println!("Assignment {} excluded successfully.", assignment_id);

    Ok(())
}

async fn run_todo(config: &config::Config, show_all: bool) -> Result<()> {
    let progress = Progress::new();
    let mut all_courses: Vec<CanvasCourse> = progress
        .wrap(
            "Loading course list",
            fetch(
                config,
                "/api/v1/courses?enrollment_state=active&per_page=10000",
            ),
        )
        .await?;
    all_courses = all_courses
        .into_iter()
        .filter(|x| {
            !config.exclude.iter().any(|y| match y {
                Exclusion::ByClassId { class_id } => class_id == &x.id,
                _ => false,
            })
        })
        .collect();
    all_courses.sort_by_key(|x| x.name.clone());
    let all_assignments = try_join_all(all_courses.into_iter().map(|x| {
        let progress = &progress;
        async move {
            progress
                .wrap(
                    &format!("Loading assignments for {}", x.name),
                    fetch::<Vec<CanvasAssignment>>(
                        config,
                        &format!(
                            "/api/v1/courses/{}/assignments?per_page=10000&include=submission",
                            x.id
                        ),
                    ),
                )
                .await
                .map(|c| (x, c))
        }
    }))
    .await?;

    progress.finish();

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
    let mut next_submission = None;
    let mut locked_count = 0;

    let mut color_id = 0;
    let mut courses_color: HashMap<i64, String> = HashMap::new();
    let mut get_course_color = |id: i64, name: &str| {
        if let Some(s) = courses_color.get(&id) {
            return s.to_string();
        }
        let s = colorize(color_id, name);
        courses_color.insert(id, s.to_string());
        color_id += 1;
        s
    };

    for (course, assignment) in all_assignments {
        if let Some(due) = assignment.due_at {
            if show_all || should_show(config, &assignment) {
                if let Some(points) = assignment.points_possible {
                    if let Some(submission) = &assignment.submission {
                        if config.hide_locked && assignment.locked_for_user {
                            locked_count += 1;
                        } else {
                            println!(
                                "{}",
                                format!(
                                    "Due {} ({}) - {}{}",
                                    if due < now {
                                        format_datetime(due).red().bold()
                                    } else {
                                        format_datetime(due).bold()
                                    },
                                    format_duration_full(now, due),
                                    get_course_color(course.id, &course.name),
                                    if submission.submitted_at.is_some() {
                                        " (completed)".white()
                                    } else {
                                        "".white()
                                    }
                                )
                                .underline()
                            );
                            let (submission_text, online_submission) =
                                process_submission(&assignment, points);
                            println!(
                                "  {} {}",
                                assignment.name.trim(),
                                format!("({})", submission_text).bright_black()
                            );
                            println!("  {}", assignment.html_url);
                            println!();
                            if due > now && submission.submitted_at.is_none() {
                                next_assignment = Some(assignment.clone());
                                if online_submission {
                                    next_submission = Some(assignment);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if locked_count != 0 {
        println!(
            "{}",
            format!(
                "(+{} locked assignment{})",
                locked_count,
                if locked_count == 1 { "" } else { "s" }
            )
            .bright_black()
        );
        println!();
    }

    if let Some(next_assignment) = next_assignment {
        println!(
            "Next assignment is due in {}",
            format_duration(now, next_assignment.due_at.unwrap())
        )
    };

    if let Some(next_submission) = next_submission {
        println!(
            "Next online submission is due in {}",
            format_duration(now, next_submission.due_at.unwrap())
        )
    };

    Ok(())
}
