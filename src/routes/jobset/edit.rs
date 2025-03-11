use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::{
    components::go_back::GoBack,
    models::{Jobset, JobsetDiff},
    routes::jobset::{self, get_jobset},
};

#[cfg(feature = "ssr")]
use {
    crate::state::State,
    leptos_axum::redirect,
    std::sync::Arc,
    tracing::{error, info},
};

#[server]
pub async fn update_jobset(jobset: Jobset) -> Result<(), ServerFnError> {
    info!("Received update");
    let state: Arc<State> = expect_context();

    let coordinator = state.coordinator.lock().await;

    let server_jobset = Jobset::get_single(
        &*coordinator.get_db().await.lock().await,
        jobset.id.unwrap(),
    )
    .await
    .map_err(|e| {
        error!("Failed to get jobset: {}", e.to_string());
        ServerFnError::new(format!("Failed to get jobset: {}", e.to_string()))
    })?;

    if server_jobset.is_none() {
        return Err(ServerFnError::new(format!("Failed to find the jobset")));
    }

    let mut server_jobset = server_jobset.unwrap();

    let mut diff = JobsetDiff::new();

    diff.set_name(jobset.name);
    diff.set_flake(jobset.flake);
    diff.set_description(jobset.description);
    diff.set_check_interval(jobset.check_interval);

    _ = server_jobset
        .update_jobset(&*coordinator.get_db().await.lock().await, diff)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    redirect(&format!(
        "/project/{}/jobset/{}",
        jobset.project_id,
        jobset.id.unwrap()
    ));
    Ok(())
}

#[component]
pub fn EditJobset() -> impl IntoView {
    let params = use_params_map();

    let project_id_str = params.read_untracked().get("proj-id").unwrap_or_default();

    let jobset_id_str = params.read_untracked().get("jobset-id").unwrap_or_default();
    let jobset_resource = OnceResource::new(get_jobset(jobset_id_str.clone()));

    let update_jobset_action = ServerAction::<UpdateJobset>::new();

    view! {
        <GoBack url=format!("/project/{}/jobset/{}", project_id_str, jobset_id_str) text="jobset".to_string()/>
        <Suspense fallback=move || view!{<p>"Loading jobset..."</p>}>
            {move || {
                let jobset = jobset_resource.get();
                if jobset.is_none() {
                    return view!{<p class="error left">"Failed to load jobset!"</p>}.into_any();
                }
                let jobset = jobset.unwrap();
                if jobset.is_err() {
                    return view!{<p class="error left">"Failed to load jobset!"</p>}.into_any();
                }

                let jobset = jobset.unwrap();

                if jobset.is_none() {
                    return view!{<p class="error left">"Failed to load jobset!"</p>}.into_any();
                }

                let jobset = jobset.unwrap();

                view!{

                    <div class="generic_input_form">
                        <ActionForm action=update_jobset_action>
                            <h3>Create a new Jobset</h3>
                            <div class="inputs">
                                <input type="hidden" name="jobset[id]" value=jobset.id.unwrap()/>
                                <input type="hidden" name="jobset[project_id]" value=jobset.project_id/>
                                <input type="text" name="jobset[name]" id="jobset_name" placeholder="Jobset Name" value=jobset.name/>
                                <input type="text" name="jobset[description]" id="jobset_desc" placeholder="Jobset Description" value=jobset.description/>
                                <input type="text" name="jobset[flake]" id="jobset_flake_uri" placeholder="Jobset Flake Uri" value=jobset.flake/>
                                <label for="jobset_check_interval">"Jobset check interval"</label>
                                <input type="number" name="jobset[check_interval]" id="jobset_check_interval" placeholder="Jobset check interval" value=jobset.check_interval/>
                                <input type="submit" value="Update jobset"/>
                            </div>
                        </ActionForm>
                    </div>
                    <div class="generic_input_form_response">
                        {move || match update_jobset_action.value().get() {
                            Some(Err(e)) => {
                                let msg = match e {
                                    ServerFnError::ServerError(msg) => msg,
                                    _ => e.to_string(),
                                };

                                view! {<p class="error">"Failed to update jobset: "{msg}</p>}.into_any()

                            },
                            _ => {

                                view! {<p class="success">""</p>}.into_any()
                            }
                        }}
                    </div>
                }.into_any()
            }}
        </Suspense>
    }
    .into_any()
}
