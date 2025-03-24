use std::{cmp::Ordering, sync::Arc};

stylance::import_crate_style!(style, "style/evaluation.module.scss");

use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use tracing::error;

use crate::{
    components::{
        error::{mk_err_view_string, mk_error_view},
        go_back::GoBack,
    },
    models::{Evaluation, JobState},
    routes::{job::get_jobs, jobset::get_jobset},
};

#[cfg(feature = "ssr")]
use crate::state::State;

#[server]
pub async fn get_evaluations(jobset_id: String) -> Result<Vec<Evaluation>, ServerFnError> {
    let state: Arc<State> = expect_context();

    let coordinator = state.coordinator.lock().await;

    let jobset_id = jobset_id
        .parse::<i32>()
        .map_err(|e| ServerFnError::new(&format!("Failed to convert to number: {}", jobset_id)))?;

    let evals = Evaluation::get_all(&*coordinator.get_db().await.lock().await, jobset_id)
        .await
        .map_err(|e| {
            error!("Failed to get evaluations: {}", e);
            ServerFnError::new("Failed to fetch evaluations!")
        })?;

    Ok(evals)
}

#[component]
pub fn Evaluation() -> impl IntoView {
    let params = use_params_map();
    let jobset_id = params.read_untracked().get("jobset-id").unwrap_or_default();
    let project_id = params.read_untracked().get("proj-id").unwrap_or_default();

    let eval_id = params.read_untracked().get("eval-id").unwrap_or_default();

    let jobs_data = OnceResource::new(get_jobs(eval_id.clone()));
    view! {
        <GoBack url=format!("/project/{}/jobset/{}", project_id, jobset_id) text="jobset".to_string()/>
        <Suspense fallback=move || view! {<p>"Loading evaluations..."</p>}>
            <div class=style::jobs>
                {move || {
                    let jobs = jobs_data.get();

                    if jobs.is_none() {
                        return mk_error_view("Failed to load jobs");
                    }

                    let jobs = jobs.unwrap();

                    if jobs.is_err() {
                        return mk_err_view_string(format!("Failed to load jobs: {}", jobs.err().unwrap().to_string()));
                    }

                    let mut jobs = jobs.unwrap();

                    if jobs.is_empty() {
                        return view! {
                            <h3>"No jobs yet!"</h3>
                        }.into_any();
                    }

                    jobs.sort_by(|a, b| {
                        if a.state == JobState::Building && b.state == JobState::Building {
                            return Ordering::Equal;
                        }

                        if a.state == JobState::Building {
                            return Ordering::Greater;
                        }

                        if b.state == JobState::Building {
                            return Ordering::Less;
                        }
                        a.finished.cmp(&b.finished)
                    });
                    jobs.reverse();

                    view!{
                        <h3>"Jobs"</h3>
                        <div class="generic_table">
                            <table>
                            <tbody>
                                <tr>
                                    <th>"Name"</th>
                                    <th>"Status"</th>
                                    <th>"Done"</th>
                                    <th>"Took"</th>
                                </tr>
                                {jobs.iter().map(|job| {
                                    let id = format!("{}", job.id.unwrap());
                                    view! {
                                        <tr>
                                            {generate_job_td(&project_id, &jobset_id, &id, &eval_id, job.attribute_name.clone())}
                                            {generate_job_td(&project_id, &jobset_id, &id, &eval_id, format!("{:#?}", job.state))}
                                            {generate_job_td(&project_id, &jobset_id, &id, &eval_id, convert_date_to_string(job.finished))}
                                            {generate_job_td(&project_id, &jobset_id, &id, &eval_id, job.took.map_or("not done yet".to_string(), |value| convert_seconds_to_minutes(value)))}
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                            </table>
                        </div>
                    }.into_any()
                }}

            </div>
    </Suspense>
    }
}

fn generate_job_td(
    project_id: &String,
    jobset_id: &String,
    job_id: &String,
    eval_id: &String,
    data: String,
) -> impl IntoView {
    let url = format!(
        "/project/{}/jobset/{}/evaluation/{}/job/{}",
        project_id, jobset_id, eval_id, job_id
    );
    view! {<td><a href=url>{data}</a></td>}.into_any()
}

fn convert_date_to_string(date: Option<DateTime<Utc>>) -> String {
    match date {
        None => "never".to_string(),
        Some(value) => value.format("%H:%M:%S %d.%m.%Y").to_string(),
    }
}

fn convert_seconds_to_minutes(seconds: i32) -> String {
    if seconds < 0 {
        return format!("{} seconds", seconds);
    }
    let mut minutes = 0;
    let mut seconds = seconds;
    while seconds >= 60 {
        minutes += 1;
        seconds -= 60;
    }

    return format!("{} minute(s) {} seconds", minutes, seconds);
}
