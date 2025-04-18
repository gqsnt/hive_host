use leptos::prelude::{ClassAttribute, IntoMaybeErased, ServerFnError};
use leptos::prelude::ElementChild;
use leptos::{component, server, view, IntoView};
use leptos::form::ActionForm;
use leptos::leptos_dom::log;
use leptos::server::ServerAction;
use crate::app::components::csrf_field::CSRFField;
use crate::models::Project;

#[component]
pub fn NewProjectPage(create_project_action: ServerAction<CreateProject>) -> impl IntoView {
    view! {
        <div>
            <h2>"New Project"</h2>
            <ActionForm action=create_project_action>
                <CSRFField />
                <div class="flex flex-col">
                    <input type="text" name="name" value="" />
                    <button type="submit">"Create Project"</button>
                </div>

            </ActionForm>
        </div>
    }
}


#[server]
pub async fn create_project(
    csrf: String,
    name:String,
) -> Result<(), ServerFnError> {
    use crate::security::utils::ssr::verify_easy_hash;
    if name.is_empty(){
        return Err(ServerFnError::ServerError(
            "Project name cannot be empty".to_string(),
        ));
    }
    let auth = crate::ssr::auth(false)?;
    let server_vars = crate::ssr::server_vars()?;
    verify_easy_hash(
        auth.session.get_session_id().to_string(),
        server_vars.csrf_server.to_secret(),
        csrf,
    )?;
    let user_slug = crate::security::utils::ssr::get_auth_session_user_slug(&auth).unwrap();
    match crate::projects::ssr::create_project(user_slug, name).await{
        Ok(project) => {
            log!("Project created: {:?}", project);
            leptos_axum::redirect(format!("/user/projects/{}", project.get_slug().to_str()).as_str());
        }
        Err(e) => {
            log!("Error creating default project: {:?}", e);
        }
    }
    
    Ok(())
}