use crate::models::Jobset;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[server]
pub async fn create_jobset(project_id: i32, jobset: Jobset) -> Result<(), ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::ResponseOptions;
    use std::sync::Arc;
    use tracing::{error, info, warn};

    let response_opts: ResponseOptions = expect_context();

    if jobset.name == "" {
        let err = "Name may not be empty!";
        warn!(err);
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new(err));
    }

    if jobset.flake == "" {
        let err = "Flake URI may not be empty!";
        warn!(err);
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new(err));
    }
    info!("Creating new jobset on project {}", project_id);

    let state: Arc<State> = expect_context();

    let result = state
        .coordinator
        .lock()
        .await
        .add_jobset(project_id, jobset)
        .await;

    if result.is_err() {
        let err = result.err().unwrap().to_string();

        error!("Failed to add jobset: {}", err);
        return Err(ServerFnError::new("Failed to add jobset!".to_string()));
    }
    leptos_axum::redirect(&format!("/project/{}", project_id));
    Ok(())
}

#[component]
pub fn CreateJobset() -> impl IntoView {
    let create_jobset_action = ServerAction::<CreateJobset>::new();

    let resp = create_jobset_action.value();

    let params = use_params_map();

    let project = params.read_untracked().get("name").unwrap_or_default();

    let has_error = move || resp.with(|val| matches!(val, Some(Err(_))));

    view! {
        <div class="generic-input-form">
            <ActionForm action=create_jobset_action>
                <h3>Create a new Jobset</h3>
                <div class="inputs">
                    <input type="hidden" name="project_id" value=project/>
                    <input type="text" name="jobset[name]" id="jobset_name" placeholder="Jobset Name"/>
                    <input type="text" name="jobset[description]" id="jobset_desc" placeholder="Jobset Description"/>
                    <input type="text" name="jobset[flake]" id="jobset_flake_uri" placeholder="Jobset Flake Uri"/>
                    <input type="number" name="jobset[check_interval]" id="jobset_check_interval" placeholder="Jobset check interval" value=0/>
                    <input type="submit" value="Create project"/>
                </div>
            </ActionForm>
        </div>
        <div class="generic-input-form-response">
            {move || if has_error() {
                view! {<p class="error">"Failed to add jobset"</p>}
            } else {
                view! {<p class="success">""</p>}
            }}
        </div>
    }
}
