use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::Params};

use crate::models::{Jobset, Project};

use crate::routes::jobset::get_jobsets;

#[derive(Params, PartialEq)]
struct ProjectParams {
    name: Option<String>,
}

#[cfg(feature = "ssr")]
use {
    crate::state::State,
    axum::http::StatusCode,
    leptos_axum::{redirect, ResponseOptions},
    std::sync::Arc,
    tracing::error,
};

#[server]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    let state: Arc<State> = expect_context();

    let coordinator = state.coordinator.lock().await;

    let projects = Project::get_all(&*coordinator.get_db().await.lock().await).await;

    let projects = projects.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(projects)
}

#[server]
pub async fn get_project(id: String) -> Result<Option<Project>, ServerFnError> {
    let state: Arc<State> = expect_context();

    let number = id.parse::<i32>();

    if number.is_err() {
        return Err(ServerFnError::ServerError(
            "Failed to fetch project".to_string(),
        ));
    }

    let number = number.unwrap();

    let result = Project::get_single(
        &*state.coordinator.lock().await.get_db().await.lock().await,
        number,
    )
    .await;

    if result.is_err() {
        error!("Failed to fetch project: {}", result.err().unwrap());
        return Err(ServerFnError::ServerError(
            "Failed to fetch project".to_string(),
        ));
    }

    Ok(result.unwrap())
}

#[server]
pub async fn delete_project(project_id: String) -> Result<(), ServerFnError> {
    let state: Arc<State> = expect_context();

    let project = get_project(project_id.clone()).await?;

    let response_opts: ResponseOptions = expect_context();

    if project.is_none() {
        response_opts.set_status(StatusCode::NOT_FOUND);
        error!("Failed to find project: {}", project_id);
        return Err(ServerFnError::new("Failed to find project!"));
    }

    let project = project.unwrap();

    _ = project
        .delete(&*state.coordinator.lock().await.get_db().await.lock().await)
        .await
        .map_err(|e| {
            error!("Failed to delete project: {}", e.to_string());
            ServerFnError::new("Failed to delete project!")
        })?;

    redirect("/");

    Ok(())
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

    let delete_project_action = ServerAction::<DeleteProject>::new();

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
                    let proj_id = project.clone();
                    view!{
                        <h4 class="title">"Project " {data.name.clone()}</h4>
                        <div class="dropdown">
                            <div class="title">
                                <span>Actions</span>
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-caret-down" viewBox="0 0 16 16">
                                  <path d="M3.204 5h9.592L8 10.481zm-.753.659 4.796 5.48a1 1 0 0 0 1.506 0l4.796-5.48c.566-.647.106-1.659-.753-1.659H3.204a1 1 0 0 0-.753 1.659"/>
                                </svg>
                            </div>
                            <div class="dropdown_content">
                                <div class="dropdown_group">
                                    <a href=format!("{}/create-jobset", proj_id)>"Create jobset"</a>
                                </div>
                                <div class="dropdown_group">
                                    <div class="generic_input_form">
                                        <ActionForm action=delete_project_action>
                                            <div class="inputs">
                                                <input type="hidden" name="project_id" value=proj_id/>
                                                <input type="submit" value="Delete project"/>
                                            </div>
                                        </ActionForm>
                                    </div>
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
