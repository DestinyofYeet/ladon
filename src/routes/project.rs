use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::Params};

use crate::models::{Jobset, Project};

use crate::routes::get_jobsets;

#[derive(Params, PartialEq)]
struct ProjectParams {
    name: Option<String>,
}

#[server]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    use crate::state::State;
    let state: Arc<State> = expect_context();

    let coordinator = state.coordinator.lock().await;

    let projects = coordinator.get_projects().await;

    let projects = projects.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(projects)
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

fn make_td_entry(proj_id: &str, id: &i32, string: &str) -> impl IntoView {
    let url = format!("/project/{}/jobset/{}", proj_id, id);
    view! {
         <td><a href=url>{string.to_string()}</a></td>
    }
}

#[component]
pub fn Project() -> impl IntoView {
    let params = use_params_map();

    let project = params.read_untracked().get("proj-id").unwrap_or_default();

    let jobsets = OnceResource::new(get_jobsets(project.clone()));

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
                            <div class="dropdown_content">
                                <div class="dropdown_group">
                                    <a href=format!("{}/create-jobset", project)>"Create jobset"</a>
                                </div>
                            </div>
                        </div>
                        <Suspense fallback=move || view! { <p>"Loading jobsets..."</p>}>

                            {move || {
                                let jobsets = jobsets.get();

                                if jobsets.is_none() {
                                    return view! {<p class="left error">"Failed to load jobsets"</p>}.into_any();
                                }

                                let jobsets = jobsets.unwrap();

                                if jobsets.is_err() {
                                    let err = jobsets.err().unwrap().to_string();
                                    return view! {<p class="left error">"Failed to load jobsets: " {err}</p>}.into_any();
                                }

                                let jobsets = jobsets.unwrap();

                                if jobsets.len() == 0 {
                                    return view! {<p class="left">"There are no jobsets yet!"</p>}.into_any();
                                }

                                view! {
                                    <p class="left">The project has following jobsets:</p>
                                    <div class="generic_table">
                                        <table>
                                            <tbody>
                                                <tr>
                                                    <th>Name</th>
                                                    <th>Description</th>
                                                </tr>
                                                    {jobsets.iter().map(|jobset| {
                                                        let job_id = jobset.id.unwrap();
                                                        view! {
                                                        <tr>
                                                            {make_td_entry(&project, &job_id, &jobset.name)}
                                                            {make_td_entry(&project, &job_id, &jobset.description)}
                                                        </tr>
                                                    }}).collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            }}

                        </Suspense>
                    }.into_any()
                }
             }
            </div>
        </Await>
    }
}
