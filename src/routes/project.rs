use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::Params};

use crate::models::Project;

#[derive(Params, PartialEq)]
struct ProjectParams {
    name: Option<String>,
}

#[server]
pub async fn get_project(id: String) -> Result<Option<Project>, ServerFnError> {
    use crate::state::State;
    use std::sync::Arc;
    use tracing::error;

    let state: Arc<State> = expect_context();

    let number = id.parse::<i32>();

    if number.is_err() {
        return Err(ServerFnError::ServerError(
            "Failed to fetch project".to_string(),
        ));
    }

    let number = number.unwrap();

    let result = state.coordinator.lock().await.get_project(number).await;

    if result.is_err() {
        error!("Failed to fetch project: {}", result.err().unwrap());
        return Err(ServerFnError::ServerError(
            "Failed to fetch project".to_string(),
        ));
    }

    Ok(result.unwrap())
}

#[component]
pub fn Project() -> impl IntoView {
    let params = use_params_map();

    let project = params.read_untracked().get("name").unwrap_or_default();

    view! {
        <Await
                future=get_project(project.clone())
            let:data
        >
            <div class="project">
            {
                let data = data.as_ref();

                if data.is_err() || data.unwrap().is_none() {
                    view! {
                        <h1>"Failed to find project"</h1>
                    }.into_any()
                } else {
                    let data = data.unwrap().as_ref().unwrap();
                    view!{
                        <h4 class="title">"Project " {data.name.clone()}</h4>
                        <div class="dropdown">
                            <span>Actions</span>
                            <div class="dropdown-content">
                                <a href=format!("{}/create-jobset", project)>"Create jobset"</a>
                            </div>
                        </div>
                        <p class="left">The project has following jobsets:</p>
                    }.into_any()
                }
             }
            </div>
        </Await>
    }
}
