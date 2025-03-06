use leptos::prelude::*;
use leptos::{component, view, IntoView};

use crate::routes::project::get_projects;

fn make_td_entry(id: &i32, string: &str) -> impl IntoView {
    view! {
         <td><a href={"/project/".to_string() + (&format!("{}", id))}>{string.to_string()}</a></td>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="generic_table">
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
                                {make_td_entry(&project.id.unwrap(), &project.name)}
                                {make_td_entry(&project.id.unwrap(), &project.description)}
                            </tr>
                        }).collect_view()}
                    </Await>
                </tbody>
            </table>
        </div>
    }
}
