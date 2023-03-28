use std::str::FromStr;

use chrono::{DateTime, Local};
use color_eyre::{
    eyre::{eyre, Context},
    Help, Result,
};
use reqwest::Url;
use scraper::{Html, Selector};

use crate::{config, CLIENT};

#[derive(Debug, Clone)]
pub struct GradescopeCourse {
    pub shortname: String,
    pub name: String,
    pub assignment_count: usize,
    pub id: i64,
}

#[derive(Debug, Clone)]
pub struct GradescopeAssignment {
    pub name: String,
    pub submitted: bool,
    pub due_at: Option<DateTime<Local>>,
    pub link: Option<String>,
}

async fn fetch(config: &config::Config, path: &str) -> Result<String> {
    CLIENT
        .get(
            Url::from_str("https://www.gradescope.com/")
                .unwrap()
                .join(path)
                .unwrap(),
        )
        .header("Cookie", config.gradescope_cookie.as_ref().unwrap())
        .send()
        .await
        .wrap_err_with(|| eyre!("Unable to fetch {}", path))?
        .error_for_status()
        .wrap_err("Server returned error")
        .suggestion("Make sure your credentials are valid")?
        .text()
        .await
        .wrap_err("Failed to read data from server")
}

pub async fn load_courses(config: &config::Config) -> Result<Vec<GradescopeCourse>> {
    let html = fetch(config, "/").await?;
    let html = Html::parse_document(&html);
    let selector = Selector::parse(".courseBox").unwrap();
    let boxes = html.select(&selector);
    Ok(boxes
        .filter_map(|b| {
            let id: i64 = b
                .value()
                .attr("href")?
                .strip_prefix("/courses/")?
                .parse()
                .ok()?;
            let t: Vec<_> = b.text().collect();
            if let [shortname, name, assignment_count] = &t[..] {
                Some(GradescopeCourse {
                    shortname: shortname.to_string(),
                    name: name.to_string(),
                    assignment_count: assignment_count.split_whitespace().next()?.parse().ok()?,
                    id,
                })
            } else {
                None
            }
        })
        .collect())
}

pub async fn load_assignments_for_course(
    config: &config::Config,
    id: i64,
) -> Result<Vec<GradescopeAssignment>> {
    let html = fetch(config, &format!("/courses/{id}")).await?;
    let html = Html::parse_document(&html);
    let selector = Selector::parse("tbody > tr").unwrap();
    let rows = html.select(&selector);

    Ok(rows
        .filter_map(|b| {
            let texts: Vec<_> = b.text().collect();
            let selector = Selector::parse("a").unwrap();
            let link = b.select(&selector).next();
            let link = link.and_then(|x| x.value().attr("href"));

            Some(GradescopeAssignment {
                due_at: texts.iter().rev().find_map(|t| {
                    let due_at = DateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S %z");
                    let due_at = due_at.ok()?;
                    Some(due_at.into())
                }),
                name: texts.first()?.to_string(),
                submitted: texts
                    .iter()
                    .find_map(|&x| {
                        if x == "Submitted" {
                            Some(true)
                        } else if x == "No Submission" {
                            Some(false)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        let selector = Selector::parse(".submissionStatus--score").unwrap();
                        let mut score = b.select(&selector);
                        score.next().is_some()
                    }),
                link: link.map(|x| x.to_string()),
            })
        })
        .collect())
}
