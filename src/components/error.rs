use leptos::prelude::*;

stylance::import_crate_style!(style, "style/components/error.module.scss");

#[component]
pub fn ErrorView(value: String) -> impl IntoView {
    view! {<p class=style::error_p>{value}</p>}
}

pub fn mk_error_view(value: &str) -> AnyView {
    view! {<ErrorView value=value.to_string()/>}.into_any()
}

pub fn mk_err_view_string(value: String) -> AnyView {
    mk_error_view(&value).into_any()
}
