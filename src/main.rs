mod canvas_api;
mod config;
mod gradescope;
mod progress;

use crate::canvas_api::{CanvasAssignment, CanvasCourse};
use crate::config::Exclusion;
use chrono::{DateTime, Local};
use color_eyre::eyre::{ContextCompat, WrapErr};
use color_eyre::{eyre::eyre, Result, Section};
use colored::Colorize;
use config::{config_path, Inclusion};
use futures::future::try_join_all;
use gradescope::{
    load_assignments_for_course, load_courses, GradescopeAssignment, GradescopeCourse,
};
use lazy_static::lazy_static;
use progress::Progress;
use reqwest::Url;
use serde::de::DeserializeOwned;
use std::cmp::Reverse;
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
            .suggestion("Make sure your credentials are valid")?
            .bytes()
            .await
            .wrap_err("Failed to read data from server")?,
    )
    .wrap_err_with(|| eyre!("Unable to parse {}", url))
}

fn format_time(time: DateTime<Local>) -> String {
    time.format("%I:%M %P").to_string()
}

fn format_datetime(datetime: DateTime<Local>) -> String {
    let today = Local::now().date_naive();
    let time = format_time(datetime);

    if datetime.date_naive() == today {
        format!("today at {}", time)
    } else if today.succ_opt() == Some(datetime.date_naive()) {
        format!("tomorrow at {}", time)
    } else if (0..7).contains(&(datetime.date_naive() - today).num_days()) {
        format!("this {} at {}", datetime.date_naive().format("%A"), time)
    } else if (7..14).contains(&(datetime.date_naive() - today).num_days()) {
        format!("next {} at {}", datetime.date_naive().format("%A"), time)
    } else {
        format!("on {} at {}", datetime.date_naive().format("%b %d"), time)
    }
}

fn format_duration(a: DateTime<Local>, b: DateTime<Local>) -> String {
    assert!(b > a);
    if (b - a).num_hours() == 1 {
        "1 hour".into()
    } else if b - a < chrono::Duration::hours(48) {
        format!("{} hours", (b - a).num_hours())
    } else {
        format!("{} days", (b.date_naive() - a.date_naive()).num_days())
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

fn should_show(config: &config::Config, assignment: &Assignment) -> bool {
    if let Some(id) = assignment.assignment_id() {
        if config
            .include
            .contains(&Inclusion::ByAssignmentId { assignment_id: id })
        {
            return true;
        }
    }

    if let Some(due) = assignment.due_at() {
        if let Some(overdue_offset) = config.hide_overdue_after_days {
            if (Local::now() - due).num_days() > overdue_offset {
                return false;
            }
        }
        match assignment {
            Assignment::Canvas(_, assignment) => {
                if config.hide_overdue_without_submission {
                    let (_, submission) = process_submission(assignment, 0.0);
                    if !submission && (Local::now() > due) {
                        return false;
                    }
                }
            }
            Assignment::Gradescope(_, assignment) => {
                if assignment.submitted {
                    return false;
                }
            }
        }
    }

    if let Assignment::Canvas(_, assignment) = assignment {
        if let Some(submission) = &assignment.submission {
            if !(submission.submitted_at.is_none()
                || assignment.peer_reviews && submission.discussion_entries.len() < 2)
            {
                return false;
            }
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
    [s.blue(), s.yellow(), s.purple(), s.cyan(), s.red()]
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
        .or_insert(toml_edit::Item::ArrayOfTables(ArrayOfTables::default()))
        .as_array_of_tables_mut()
        .wrap_err("`exclude` is not an array of tables")?
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

#[derive(Debug)]
enum Assignment {
    Canvas(CanvasCourse, CanvasAssignment),
    Gradescope(GradescopeCourse, GradescopeAssignment),
}

impl Assignment {
    fn assignment_id(&self) -> Option<i64> {
        match self {
            Assignment::Canvas(_, a) => Some(a.id),
            Assignment::Gradescope(_, _) => None,
        }
    }

    fn due_at(&self) -> Option<DateTime<Local>> {
        match self {
            Assignment::Canvas(_, a) => a.due_at,
            Assignment::Gradescope(_, a) => a.due_at,
        }
    }
}

async fn run_todo(config: &config::Config, show_all: bool) -> Result<()> {
    let progress = Progress::new();

    let (canvas_assignments, gradescope_assignments) = tokio::try_join!(
        load_canvas(&progress, config),
        load_gradescope(&progress, config),
    )?;

    let mut all_assignments: Vec<_> = gradescope_assignments
        .into_iter()
        .flat_map(|(c, a)| a.into_iter().map(move |x| (c.clone(), x)))
        .map(|(c, a)| Assignment::Gradescope(c, a))
        .chain(
            canvas_assignments
                .into_iter()
                .flat_map(|(c, a)| a.into_iter().map(move |x| (c.clone(), x)))
                .map(|(c, a)| Assignment::Canvas(c, a)),
        )
        .collect();

    progress.finish();

    all_assignments.retain(|a| {
        !config.exclude.iter().any(|x| match a.assignment_id() {
            Some(id) => x == &Exclusion::ByAssignmentId { assignment_id: id },
            None => false,
        })
    });

    all_assignments.sort_by_key(|x| Reverse(x.due_at()));

    let now = Local::now();

    let mut next_assignment_due_at = None;
    let mut next_submission_due_at = None;
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

    for assignment in all_assignments {
        if let Some(due) = assignment.due_at() {
            if show_all || should_show(config, &assignment) {
                match assignment {
                    Assignment::Canvas(course, assignment) => {
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
                                        if let Some(due_at) = assignment.due_at {
                                            next_assignment_due_at = Some(due_at);
                                            if online_submission {
                                                next_submission_due_at = Some(due_at);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Assignment::Gradescope(course, assignment) => {
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
                                if assignment.submitted {
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
                            format!("({})", "Gradescope".purple()).bright_black()
                        );
                        if let Some(link) = assignment.link {
                            println!("  https://www.gradescope.com{}", link);
                        }
                        println!();

                        if let Some(due_at) = assignment.due_at {
                            next_assignment_due_at = Some(due_at);
                            next_submission_due_at = Some(due_at);
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

    if let Some(next_assignment_due_at) = next_assignment_due_at {
        println!(
            "Next assignment is due in {}",
            format_duration(now, next_assignment_due_at)
        )
    };

    if let Some(next_submission_due_at) = next_submission_due_at {
        println!(
            "Next online submission is due in {}",
            format_duration(now, next_submission_due_at)
        )
    };

    Ok(())
}

async fn load_canvas(
    progress: &Progress,
    config: &config::Config,
) -> Result<Vec<(CanvasCourse, Vec<CanvasAssignment>)>> {
    let mut canvas_courses: Vec<CanvasCourse> = progress
        .wrap(
            "Loading course list",
            fetch(
                config,
                "/api/v1/courses?enrollment_state=active&per_page=10000",
            ),
        )
        .await?;
    canvas_courses.retain(|x| {
        !config.exclude.iter().any(|y| match y {
            Exclusion::ByClassId { class_id } => class_id == &x.id,
            _ => false,
        })
    });
    canvas_courses.sort_by_key(|x| x.name.clone());
    let canvas_assignments = try_join_all(canvas_courses.into_iter().map(|x| {
        let progress = progress;
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
    Ok(canvas_assignments)
}

async fn load_gradescope(
    progress: &Progress,
    config: &config::Config,
) -> Result<Vec<(GradescopeCourse, Vec<GradescopeAssignment>)>> {
    if config.gradescope_cookie.is_none() {
        return Ok(vec![]);
    }

    let gradescope_courses = progress
        .wrap("Loading Gradescope courses", async move {
            load_courses(config).await
        })
        .await?;
    let gradescope_assignments: Vec<(GradescopeCourse, Vec<GradescopeAssignment>)> =
        try_join_all(gradescope_courses.into_iter().map(|x| {
            let progress = progress;
            async move {
                progress
                    .wrap(&format!("Loading assignments for {}", x.name), async move {
                        let assignments = load_assignments_for_course(config, x.id).await?;
                        Ok((x.clone(), assignments)) as Result<_>
                    })
                    .await
            }
        }))
        .await?;
    Ok(gradescope_assignments)
}
