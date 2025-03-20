use crate::models::Jobset;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[server]
pub async fn create_jobset(jobset: Jobset) -> Result<(), ServerFnError> {
    use crate::hydracore::Coordinator;
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::ResponseOptions;
    use std::sync::Arc;
    use tracing::{error, info, warn};

    let response_opts: ResponseOptions = expect_context();

    if jobset.name == "" {
        let err = "Name may not be empty!";
        warn!("{}", err);
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new(err));
    }

    if jobset.flake == "" {
        let err = "Flake URI may not be empty!";
        warn!("{}", err);
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new(err));
    }
    info!("Creating new jobset on project {}", jobset.project_id);

    let state: Arc<State> = expect_context();

    let mut jobset = jobset;

    let result = jobset
        .add_to_db(&*state.coordinator.lock().await.get_db().await.lock().await)
        .await;

    if result.is_err() {
        let err = result.err().unwrap().to_string();

        error!("Failed to add jobset: {}", err);
        return Err(ServerFnError::new("Failed to add jobset!".to_string()));
    }

    Coordinator::start_jobset_timer(state.clone(), jobset.clone());

    leptos_axum::redirect(&format!(
        "/project/{}/jobset/{}",
        jobset.project_id,
        jobset.id.unwrap()
    ));
    Ok(())
}

#[component]
pub fn CreateJobset() -> impl IntoView {
    let create_jobset_action = ServerAction::<CreateJobset>::new();

    let resp = create_jobset_action.value();

    let params = use_params_map();

    let project = params.read_untracked().get("proj-id").unwrap_or_default();

    view! {
        <div class="generic_input_form">
            <ActionForm action=create_jobset_action>
                <h3>Create a new Jobset</h3>
                <div class="inputs">
                    <input type="hidden" name="jobset[project_id]" value=project/>
                    <input type="text" name="jobset[name]" id="jobset_name" placeholder="Jobset Name"/>
                    <input type="text" name="jobset[description]" id="jobset_desc" placeholder="Jobset Description"/>
                    <input type="text" name="jobset[flake]" id="jobset_flake_uri" placeholder="Jobset Flake Uri"/>
                    <label for="jobset_check_interval">"Jobset check interval"</label>
                    <input type="number" name="jobset[check_interval]" id="jobset_check_interval" placeholder="Jobset check interval" value=0/>
                    <input type="submit" value="Create jobset"/>
                </div>
            </ActionForm>
        </div>
        <div class="generic_input_form_response">
            {move || match resp.get() {
                Some(Err(e)) => {
                    let msg = match e {
                        ServerFnError::ServerError(msg) => msg,
                        _ => e.to_string(),
                    };

                    view! {<p class="error">"Failed to add jobset: "{msg}</p>}.into_any()

                },
                _ => {

                    view! {<p class="success">""</p>}.into_any()
                }
            }}
        </div>
    }
}
