use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::models::Project;

use super::get_project;

#[cfg(feature = "ssr")]
use {crate::state::State, leptos_axum::redirect, std::sync::Arc, tracing::info};

#[server]
pub async fn update_project(project: Project) -> Result<(), ServerFnError> {
    info!("Received update");
    let state: Arc<State> = expect_context();

    let coordinator = state.coordinator.lock().await;

    let server_project = Project::get_single(
        &*coordinator.get_db().await.lock().await,
        project.id.unwrap(),
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to update project: {}", e.to_string())))?;

    if server_project.is_none() {
        return Err(ServerFnError::new("Failed to find project!"));
    }

    let mut server_project = server_project.unwrap();

    server_project.name = project.name;
    server_project.description = project.description;

    _ = server_project
        .update(&*coordinator.get_db().await.lock().await)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update project: {}", e.to_string())))?;

    redirect("/");

    Ok(())
}

#[component]
pub fn EditProject() -> impl IntoView {
    let params = use_params_map();

    let project_id_str = params.read_untracked().get("proj-id").unwrap_or_default();
    let project_resource = OnceResource::new(get_project(project_id_str.clone()));

    let update_project_action = ServerAction::<UpdateProject>::new();

    view! {
        <Suspense fallback= move || view!{<p>"Loading project..."</p>}>
            {move || {
                let project = project_resource.get();

                if project.is_none() {
                    return view!{ <p class="left error">"Failed to load project"</p>}.into_any();
                }

                let project = project.unwrap();

                if project.is_err() {
                    return view!{ <p class="left error">"Failed to load project"</p>}.into_any();
                }

                let project = project.unwrap();

                if project.is_none() {
                    return view!{ <p class="left error">"Failed to load project"</p>}.into_any();
                }

                let project = project.unwrap();

                view!{
                    <div class="generic_input_form">
                        <ActionForm action=update_project_action>
                            <h3>"Update project"</h3>
                            <div class="inputs">
                                <input type="text" name="project[name]" id="proj_name" placeholder="Project Name" value=project.name/>
                                <input type="text" name="project[description]" id="proj_desc" placeholder="Project Description" value=project.description/>
                                <input type="hidden" name="project[id]" value=project.id.unwrap()/>
                                <input type="submit" value="Update project"/>
                            </div>
                        </ActionForm>
                    </div>
                    <div class="generic_input_form_response">
                        {move || match update_project_action.value().get() {
                            Some(Err(e)) => {
                                let msg = match e {
                                    ServerFnError::ServerError(msg) => msg,
                                    _ => e.to_string(),
                                };
                                view! {<p class="error">"Failed to update project: "{msg}</p>}.into_any()
                            },
                            _ => {view! {}.into_any()},
                         }}
                    </div>
                }.into_any()
            }}
        </Suspense>
    }
}
