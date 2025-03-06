use leptos::prelude::*;
use tracing::{error, info, warn};

use crate::models::Project;

#[server]
pub async fn create_project_server(project: Project) -> Result<(), ServerFnError> {
    use crate::state::State;
    use axum::http::StatusCode;
    use leptos_axum::ResponseOptions;
    use std::sync::Arc;

    let response_opts: ResponseOptions = expect_context();

    if project.name == "" {
        let err = "Name cannot be empty!".to_string();
        warn!("{err}");
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new(err));
    } else if project.description == "" {
        let err = "Description cannot be empty!".to_string();
        warn!("{err}");
        response_opts.set_status(StatusCode::BAD_REQUEST);
        return Err(ServerFnError::new(err));
    }

    let state: Arc<State> = expect_context();

    let mut project = project;

    let result = project
        .add_to_db(&*state.coordinator.lock().await.get_db().await.lock().await)
        .await;

    if result.is_err() {
        error!("Failed to create project: {}", result.err().unwrap());
        return Err(ServerFnError::new(
            "Failed to create project! Check the server logs!",
        ));
    }

    info!("Created new project: {}", project.name);
    leptos_axum::redirect(&format!("/project/{}", project.id.unwrap()));
    Ok(())
}

#[component]
pub fn CreateProject() -> impl IntoView {
    let create_project_action = ServerAction::<CreateProjectServer>::new();

    let resp = create_project_action.value();

    view! {
        <div class="generic_input_form">
            <ActionForm action=create_project_action>
                <h3>Create a new project</h3>
                <div class="inputs">
                    <input type="text" name="project[name]" id="proj_name" placeholder="Project Name"/>
                    <input type="text" name="project[description]" id="proj_desc" placeholder="Project Description"/>
                    <input type="submit" value="Create project"/>
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
                    view! {<p class="error">"Failed to add project: "{msg}</p>}.into_any()
                },
                _ => {view! {}.into_any()},
             }}
        </div>
    }
}
