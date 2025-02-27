use leptos::{component, view, IntoView};
use leptos::prelude::*;

use crate::models::Project;

#[server]
pub async fn get_projects () -> Result<Vec<Project>, ServerFnError> {
    let mut vec = Vec::new();
    vec.push(Project::new(
        "some_id".to_string(),
        "some_name".to_string(),
        "some_desc".to_string()
    ));

    vec.push(Project::new("cool_project_123".to_string(), "cool_name".to_string(), "cool_description".to_string()));

    Ok(vec)
}

fn make_td_entry(id: &str, string: &str) -> impl IntoView {
   view! {
        <td><a href={"/project/".to_string() + id}>{string.to_string()}</a></td>
   } 
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="projects">
            <h3>Projects</h3>
            <div class="description">
            <p>The following projects are hosted here.</p>
            </div>
            <table>
                <tr>
                    <th>ID</th>
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
                            {make_td_entry(&project.id, &project.id)}
                            {make_td_entry(&project.id, &project.name)}
                            {make_td_entry(&project.id, &project.description)}
                        </tr>
                    }).collect_view()}
                </Await>
            </table>
        </div>
    }
} 
