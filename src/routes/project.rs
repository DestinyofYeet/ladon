use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::Params};

#[derive(Params, PartialEq)]
struct ProjectParams {
    name: Option<String>,
}

#[component]
pub fn Project() -> impl IntoView {
    let params = use_params_map();

    let project = move || params.read().get("name").unwrap_or_default();

    view! {
        <div class="project">
            <h4 class="title">"Project "{project}</h4>
        </div>
    }
}
