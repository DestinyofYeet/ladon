use leptos::prelude::*;

stylance::import_crate_style!(style, "style/components/go_back.module.scss");

#[component]
pub fn GoBack(url: String, text: String) -> impl IntoView {
    view! {
        <div class=style::go_back>
            <a href=url>"Go back to "{text}</a>
        </div>
    }
}
