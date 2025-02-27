use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="notfound">
            <h1>Sorry, we were not able to find the page you were looking for.</h1>
        </div>
    }
}
