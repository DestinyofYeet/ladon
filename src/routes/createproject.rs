use leptos::prelude::*;

use leptos_router::components::Form;

#[component]
pub fn CreateProject() -> impl IntoView {
    view! {
        <div class="create-project-form">
            <Form method="POST" action="">
                <input type="text" name="proj_name" id="proj_name" placeholder="Project Name"/>
                <input type="text" name="proj_desc" id="proj_desc" placeholder="Project Description"/>
            </Form>
        </div>
    }
}
