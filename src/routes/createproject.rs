use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectCreationData {
    pub name: String,
    pub description: String,
}

#[server]
pub async fn create_project_server(proj_data: ProjectCreationData) -> Result<(), ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::ResponseOptions;
    use std::sync::Arc;

    let response_opts: ResponseOptions = expect_context();

    let name = proj_data.name.trim();
    let desc = proj_data.description.trim();

    if name == "" {
        let err = "Name cannot be empty!".to_string();
        warn!("{err}");
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::MissingArg(err));
    } else if desc == "" {
        let err = "Description cannot be empty!".to_string();
        warn!("{err}");
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::MissingArg(err));
    }

    let state: Arc<State> = expect_context();

    let result = state.coordinator.lock().await.add_project(name, desc).await;
    if result.is_err() {
        error!("Failed to create project: {}", result.err().unwrap());
        return Err(ServerFnError::new(""));
    }

    info!("Created new project: {}", name);
    leptos_axum::redirect("/");
    Ok(())
}

#[component]
pub fn CreateProject() -> impl IntoView {
    let create_project_action = ServerAction::<CreateProjectServer>::new();

    let resp = create_project_action.value();

    let has_error = move || resp.with(|val| matches!(val, Some(Err(_))));

    view! {
        <div class="generic-input-form">
            <ActionForm action=create_project_action>
                <h3>Create a new project</h3>
                <div class="inputs">
                    <input type="text" name="proj_data[name]" id="proj_name" placeholder="Project Name"/>
                    <input type="text" name="proj_data[description]" id="proj_desc" placeholder="Project Description"/>
                    <input type="submit" value="Create project"/>
                </div>
            </ActionForm>
        </div>
        <div class="generic-input-form-response">
            {move || if has_error() {
                view! {<p class="error">"Failed to add project"</p>}
            } else {
                view! {<p class="success">""</p>}
            }}
        </div>
    }
}
