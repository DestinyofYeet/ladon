use std::sync::Arc;

use leptos::prelude::*;
use leptos::{component, view, IntoView};

use crate::models::Project;

#[server]
pub async fn get_projects() -> Result<Vec<Project>, ServerFnError> {
    use crate::state::State;
    let state: Arc<State> = expect_context();

    let coordinator = state.coordinator.lock().await;

    let projects = coordinator.get_projects().await;

    let projects = projects.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(projects)
}

fn make_td_entry(id: &i32, string: &str) -> impl IntoView {
    view! {
         <td><a href={"/project/".to_string() + (&format!("{}", id))}>{string.to_string()}</a></td>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="projects-table">
            <h3>Projects</h3>
            <div class="description">
            <p>The following projects are hosted here.</p>
            </div>
            <table>
                <tbody>
                    <tr>
                        <th>Name</th>
                        <th>Description</th>
                    </tr>
                    <Await
                        future=get_projects()
                        let:data
                    >
                        {data.as_ref().unwrap().into_iter().map(|project|
                            view! {
                            <tr>
                                {make_td_entry(&project.id, &project.name)}
                                {make_td_entry(&project.id, &project.description)}
                            </tr>
                        }).collect_view()}
                    </Await>
                </tbody>
            </table>
        </div>
    }
}
