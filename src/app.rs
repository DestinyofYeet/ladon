use std::sync::Arc;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes}, path, StaticSegment
};

use leptos::task::spawn_local;

use tracing::info;

use crate::{routes, state};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[server]
pub async fn send_value(value: i32) -> Result<(), ServerFnError> {
    let app_state = expect_context::<Arc<state::State>>();
    println!("Received value: {}", value);

    *app_state.value.lock().unwrap() += 1;
    Ok(())
}

#[server]
pub async fn get_value() -> Result<i32, ServerFnError> {
    let app_state = expect_context::<Arc<state::State>>();

    let locked = app_state.value.lock().unwrap();

    Ok(*locked)
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/hydra-rs.css"/>

        // sets the document title
        <Title text="Hydra-rs"/>

        // content for this welcome page
        {HomePage()}
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    //// Creates a reactive value to update the button
    //let once = OnceResource::new(get_value());
    //let send_value_action = Action::new(|input: &i32| {
    //    let input = input.clone();
    //    async move {
    //        _ = send_value(input).await;
    //    }
    //});
    //
    //let count = RwSignal::new(0);
    //let on_click = move |_| {
    //    *count.write() += 1;
    //
    //    send_value_action.dispatch(count.get());
    //};
    //
    //Effect::new(move |_|  {
    //    if let Some(value) = once.get() {
    //        count.set(value.expect("Failed to get value from server"));
    //    }
    //});

    //view! {
    //    <h1>"Welcome to Leptos!"</h1>
    //    <button on:click=on_click>"Click Me: " {count}</button>
    //}
    

    view! {
        <Router>
            <nav> // navbar
                <div class="topnav">
                    <a class="hydra" href="/">Hydra-rs</a>
                    <div class="entries">
                        <a href="jobsets">Jobsets</a>
                        <a href="dashboard">Dashboard</a>
                    </div>
                </div>
            </nav>
            <main>
                <Routes fallback=|| routes::NotFound>
                    <Route path=path!("/") view=routes::Home/>
                </Routes>
            </main>
        </Router>
    }
}
