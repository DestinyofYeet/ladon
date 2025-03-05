use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
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

//#[server]
//pub async fn send_value(value: i32) -> Result<(), ServerFnError> {
//    let app_state = expect_context::<Arc<state::State>>();
//    println!("Received value: {}", value);
//
//    *app_state.value.lock().unwrap() += 1;
//    Ok(())
//}

//#[server]
//pub async fn get_value() -> Result<i32, ServerFnError> {
//    let app_state = expect_context::<Arc<state::State>>();
//
//    let locked = app_state.value.lock().unwrap();
//
//    Ok(*locked)
//}

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
                        <div class="dropdown">
                            <div class="title">
                                <span>Admin</span>
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-caret-down" viewBox="0 0 16 16">
                                  <path d="M3.204 5h9.592L8 10.481zm-.753.659 4.796 5.48a1 1 0 0 0 1.506 0l4.796-5.48c.566-.647.106-1.659-.753-1.659H3.204a1 1 0 0 0-.753 1.659"/>
                                </svg>
                            </div>
                            <div class="dropdown_content">
                                <div class="dropdown_group">
                                    <a href="/create-project">Create Project</a>
                                    <a href="/blub-blub">Blub blub</a>
                                </div>
                                <div class="dropdown_group">
                                    <a href="/somethingelse">Something else</a>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </nav>
            <main>
                <Routes fallback=|| routes::NotFound>
                    //<ParentRoute path=path!("/project") view=routes::NotFound>
                    //    <Route path=path!("/project/:name") view=routes::Project/>
                    //    <Route path=path!("") view=|| {println!("Tried accessing /project. "); routes::NotFound}/> // dunno if needed
                    //</ParentRoute>
                    <Route path=path!("/") view=routes::Home/>
                    <Route path=path!("/create-project") view=routes::CreateProject/>
                    <Route path=path!("/project/:proj-id") view=routes::Project/>
                    <Route path=path!("/project/:proj-id/create-jobset") view=routes::jobsets::CreateJobset/>
                    <Route path=path!("/project/:proj-id/jobset/:jobset-id") view=routes::jobsets::Jobset/>
                </Routes>
            </main>
        </Router>
    }
}
