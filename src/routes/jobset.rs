use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn Jobset() -> impl IntoView {
    let params = use_params_map();

    let project_id = params.read_untracked().get("proj-id").unwrap_or_default();
    let jobset_id = params.read_untracked().get("jobset-id").unwrap_or_default();

    view! {
        <p>"Hello from Jobset " {jobset_id} " on project " {project_id}</p>
    }
}
