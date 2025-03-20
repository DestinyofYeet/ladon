use std::{process::Stdio, str::FromStr};

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::{components::go_back::GoBack, models::Job};

stylance::import_crate_style!(style, "style/job.module.scss");

#[cfg(feature = "ssr")]
use {
    crate::state::State,
    axum::http::StatusCode,
    leptos_axum::{redirect, ResponseOptions},
    std::sync::Arc,
    tokio::process::Command,
    tracing::{error, info, trace},
};

#[server]
pub async fn get_job_output(job_id: String) -> Result<String, ServerFnError> {
    let job = get_job(job_id).await?;

    let response_opts: ResponseOptions = expect_context();

    if job.is_none() {
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new("Failed to fetch job!"));
    }

    let job = job.unwrap();

    let command = Command::new("nix")
        .arg("log")
        .arg(job.derivation_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let command = command.map_err(|e| {
        error!("Failed to spawn nix log: {}", e.to_string());
        ServerFnError::new("Failed to get log!")
    })?;

    let result = command.wait_with_output().await.map_err(|e| {
        error!("Failed to wait for nix log: {}", e.to_string());
        ServerFnError::new("Failed to get log!")
    })?;

    let stdout = String::from_utf8(result.stdout).unwrap();

    // trace!("stdout:\n{stdout}");

    Ok(stdout)
}

#[server]
pub async fn get_job(job_id: String) -> Result<Option<Job>, ServerFnError> {
    let number = job_id.parse::<i32>();

    if number.is_err() {
        return Err(ServerFnError::ServerError(
            "Failed to fetch job".to_string(),
        ));
    }

    let number = number.unwrap();

    let state: Arc<State> = expect_context();

    let db = state.coordinator.lock().await.get_db().await;

    let db_locked = db.lock().await;

    let job = Job::get_single(&*db_locked, number)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(job)
}

#[server]
pub async fn get_jobs(jobset_id: String) -> Result<Vec<Job>, ServerFnError> {
    let state: Arc<State> = expect_context();

    let db = state.coordinator.lock().await.get_db().await;

    let db_locked = db.lock().await;

    let jobs = Job::get_all(&*db_locked, jobset_id.parse().unwrap()).await;

    let jobs = jobs.map_err(|e| {
        error!("Failed to get jobs: {}", e.to_string());
        ServerFnError::new("Failed to get jobsets!")
    })?;

    Ok(jobs)
}

#[component]
pub fn Job() -> impl IntoView {
    let params = use_params_map();

    let project_id = params.read_untracked().get("proj-id").unwrap_or_default();
    let jobset_id = params.read_untracked().get("jobset-id").unwrap_or_default();
    let job_id = params.read_untracked().get("job-id").unwrap_or_default();

    let output_data = OnceResource::new(get_job_output(job_id.clone()));

    let back_url = format!("/project/{}/jobset/{}", project_id, job_id);

    view! {
        <div class=style::job>
        <GoBack url=back_url text="jobset".to_string()/>
        <Suspense fallback=move || view!{<p>"Loading job log..."</p>}>
            {move || {
                let output = output_data.get();

                if output.is_none() {
                    return view! {<p class="left error">"Failed to get log"</p>}.into_any();
                }

                let output = output.unwrap();

                if output.is_err(){
                    return view! {<p class="error left">"Error: Failed to get output: "{output.err().unwrap().to_string()}</p>}.into_any();
                }

                let mut output = output.unwrap();

                if output.is_empty() {
                    output = String::from_str("The log is empty :/").unwrap();
                }

                view!{<pre class=style::log>{output}</pre>}.into_any()
            }}
        </Suspense>
        </div>
    }
}
