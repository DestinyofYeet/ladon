use chrono::{DateTime, Utc};
use leptos::{prelude::*, server_fn::ServerFn, task::spawn_local};
use leptos_router::hooks::use_params_map;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{
    components::go_back::GoBack,
    models::{Jobset, JobsetState},
};

stylance::import_crate_style!(
    #[allow(dead_code)]
    style,
    "style/jobset.module.scss"
);

#[server]
pub async fn delete_jobset(project_id: String, jobset_id: String) -> Result<(), ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::{redirect, ResponseOptions};
    use std::sync::Arc;
    use tracing::error;

    let state: Arc<State> = expect_context();
    let response_opts: ResponseOptions = expect_context();

    let jobset = get_jobset(jobset_id).await?;

    if jobset.is_none() {
        response_opts.set_status(StatusCode::NOT_FOUND);
        return Err(ServerFnError::new("Failed to find jobset!"));
    }

    let mut jobset = jobset.unwrap();

    _ = jobset
        .delete(&*state.coordinator.lock().await.get_db().await.lock().await)
        .await
        .map_err(|e| {
            error!("Failed to delete jobset: {}", e.to_string());
            ServerFnError::new("Failed to delete jobset!")
        })?;

    redirect(&format!("/project/{}", project_id));

    Ok(())
}

#[server]
pub async fn get_jobsets(id: String) -> Result<Vec<Jobset>, ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::ResponseOptions;
    use std::sync::Arc;
    use tracing::error;

    let state: Arc<State> = expect_context();
    let response_opts: ResponseOptions = expect_context();

    let number = id.parse::<i32>();

    if number.is_err() {
        response_opts.set_status(StatusCode::BAD_REQUEST);
        error!("Invalid project id given");
        return Err(ServerFnError::new("Failed to find project!"));
    }

    let number = number.unwrap();

    let jobsets = Jobset::get_all(
        &*state.coordinator.lock().await.get_db().await.lock().await,
        number,
    )
    .await;

    if jobsets.is_err() {
        error!(
            "Failed to fetch jobsets: {}",
            jobsets.err().unwrap().to_string()
        );
        return Err(ServerFnError::new("Failed to fetch jobsets"));
    }

    let jobsets = jobsets.unwrap();

    Ok(jobsets)
}

#[server]
pub async fn get_jobset(id: String) -> Result<Option<Jobset>, ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::ResponseOptions;
    use std::sync::Arc;
    use tracing::error;

    let state: Arc<State> = expect_context();
    let response_opts: ResponseOptions = expect_context();

    let jobset_id = id.parse::<i32>();

    if jobset_id.is_err() {
        response_opts.set_status(StatusCode::BAD_REQUEST);
        error!("Invalid jobset given");
        return Err(ServerFnError::new("Failed to find jobset!"));
    }

    let jobset_id = jobset_id.unwrap();

    let jobset = Jobset::get_single(
        &*state.coordinator.lock().await.get_db().await.lock().await,
        jobset_id,
    )
    .await;

    if jobset.is_err() {
        error!("Failed to fetch jobset: {}", jobset.err().unwrap());
        return Err(ServerFnError::new("Failed to fetch jobset!"));
    }

    Ok(jobset.unwrap())
}

#[server]
pub async fn trigger_jobset(project_id: String, jobset_id: String) -> Result<(), ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::{redirect, ResponseOptions};
    use std::sync::Arc;
    use tracing::error;
    use tracing::info;

    let jobset = get_jobset(jobset_id.clone()).await?;
    let response_opts: ResponseOptions = expect_context();

    if jobset.is_none() {
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new("Failed to find jobset!"));
    }

    let mut jobset = jobset.unwrap();

    let state: Arc<State> = expect_context();

    info!("Triggered jobset: {}", jobset_id);

    let result = state
        .coordinator
        .lock()
        .await
        .schedule_jobset(&mut jobset)
        .await;

    if result.is_err() {
        let err = result.err().unwrap().to_string();
        error!("Failed to schedule jobset: {}", err);
        return Err(ServerFnError::new(err));
    }

    Ok(())
}

#[component]
pub fn Jobset() -> impl IntoView {
    let params = use_params_map();

    let project_id = params.read_untracked().get("proj-id").unwrap_or_default();
    let jobset_id = params.read_untracked().get("jobset-id").unwrap_or_default();

    let (input, _set_input) = signal(jobset_id.clone());

    let jobset_data = Resource::new(
        move || (input.get()),
        |input| async move { get_jobset(input).await },
    );

    let trigger_jobset_action = ServerAction::<TriggerJobset>::new();
    let delete_jobset_action = ServerAction::<DeleteJobset>::new();

    Effect::new(move |_| {
        if let Some(Ok(_)) = trigger_jobset_action.value().get() {
            jobset_data.refetch();
        }
    });

    view! {
        <GoBack url=format!("/project/{}", project_id) text="project".to_string()/>
        <Suspense fallback=move || view! {<p>"Loading jobset data..."</p>}>
            {move || {
                let jobset = jobset_data.get();

                if jobset.is_none() {
                    return view! {<p>"Error: Failed to load jobset!"</p>}.into_any();
                }

                let jobset = jobset.unwrap();

                if jobset.is_err() {
                    let e = jobset.err().unwrap();
                    let msg = match e {
                        ServerFnError::ServerError(msg) => msg,
                        _ => e.to_string(),
                    };
                    return view! {<p class="error">"Error: Failed to load jobset: "{msg}</p>}.into_any();
                }

                let jobset = jobset.unwrap();

                if jobset.is_none() {
                    return view!{<p>"Error: Failed to find jobset!"</p>}.into_any();
                }

                let jobset = jobset.unwrap();

                view! {
                    <div class=style::view>
                        <div class=style::action>
                            <div class="dropdown">
                                <div class="title">
                                    <span>Actions</span>
                                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-caret-down" viewBox="0 0 16 16">
                                      <path d="M3.204 5h9.592L8 10.481zm-.753.659 4.796 5.48a1 1 0 0 0 1.506 0l4.796-5.48c.566-.647.106-1.659-.753-1.659H3.204a1 1 0 0 0-.753 1.659"/>
                                    </svg>
                                </div>
                                <div class="dropdown_content">
                                    <div class="dropdown_group">
                                        <a href=format!("/project/{}/jobset/{}/edit", project_id, jobset_id)>"Edit jobset"</a>
                                    </div>
                                    <div class="dropdown_group">
                                        <div class="generic_input_form">
                                            <ActionForm action=trigger_jobset_action>
                                                <div class="inputs">
                                                    <input type="hidden" name="project_id" value=jobset.project_id.to_string()/>
                                                    <input type="hidden" name="jobset_id" value=jobset.id.unwrap().to_string()/>
                                                    <input type="submit" value="Trigger jobset"/>
                                                </div>
                                            </ActionForm>
                                        </div>
                                    </div>
                                    <div class="dropdown_group">
                                        <div class="generic_input_form">
                                           <ActionForm action=delete_jobset_action>
                                                <div class="inputs">
                                                    <input type="hidden" name="project_id" value=jobset.project_id.to_string()/>
                                                    <input type="hidden" name="jobset_id" value=jobset.id.unwrap().to_string()/>
                                                    <input type="submit" value="Delete jobset"/>
                                                </div>
                                           </ActionForm>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div class=style::trigger_result>
                            {move || {
                                match trigger_jobset_action.value().get() {
                                    Some(Err(e)) => {
                                        let msg = match e {
                                            ServerFnError::ServerError(msg) => msg,
                                            _ => e.to_string(),
                                        };

                                        return view! {
                                            <p class="failed">"Failed to trigger jobset: "{msg}</p>
                                        }.into_any();
                                    },

                                    None => {
                                        return view! {
                                        }.into_any();
                                    }

                                    _ => {
                                       return view! {
                                            <p class="success">"Successfully triggered jobset"</p>
                                       }.into_any();
                                    }
                                }

                                match delete_jobset_action.value().get() {
                                    Some(Err(e)) => {
                                        let msg = match e {
                                            ServerFnError::ServerError(msg) => msg,
                                            _ => e.to_string(),
                                        };

                                        return view! {
                                            <p class="failed">"Failed to delete jobset: "{msg}</p>
                                        }.into_any();
                                    },

                                    None => return view!{}.into_any(),

                                    _ => return view!{}.into_any(),
                                }
                            }}
                        </div>
                        <div class=style::statistics>
                            {mk_jobset_entry("Name: ", jobset.name)}
                            {mk_jobset_entry("Description: ", jobset.description)}
                            {mk_jobset_entry("Flake URI: ", jobset.flake)}
                            {mk_jobset_entry("Last checked: ", convert_date_to_string(jobset.last_checked))}
                            {mk_jobset_entry("Last evaluated: ", convert_date_to_string(jobset.last_evaluated))}
                            {mk_jobset_entry("Check interval (every): ", convert_seconds_to_minutes(jobset.check_interval))}
                            {mk_jobset_entry("Evaluation took: ", convert_seconds_to_minutes(jobset.evaluation_took.unwrap_or(-1)))}
                            {mk_jobset_entry("State: ", jobset.state.clone().unwrap_or(JobsetState::Unknown).to_string())}
                            {
                                match jobset.state {
                                    Some(JobsetState::EvalFailed) => mk_jobset_entry("Error", jobset.error_message.unwrap()).into_any(),
                                    _ => view!{}.into_any()
                                }
                            }
                        </div>
                    </div>
                }.into_any()
            }}
        </Suspense>
    }
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

fn mk_jobset_entry(key: &str, value: String) -> impl IntoView {
    view! {
        <div class=style::key>
            <p>{key.to_string()}</p>
        </div>
        <div class=style::value>
            <p>{value}</p>
        </div>
    }
}
