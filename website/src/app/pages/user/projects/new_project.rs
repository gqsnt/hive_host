use leptos::prelude::{ClassAttribute, IntoMaybeErased, ServerFnError};
use leptos::prelude::ElementChild;
use leptos::{component, server, view, IntoView};
use leptos::form::ActionForm;
use leptos::leptos_dom::log;
use leptos::server::ServerAction;
use crate::app::components::csrf_field::CSRFField;
use crate::models::Project;
use leptos::prelude::IntoAnyAttribute;


#[component]
pub fn NewProjectPage(create_project_action: ServerAction<CreateProject>) -> impl IntoView {
    // Reuse styling from settings page
    let input_class = "block w-full rounded-md bg-white/5 px-3 py-1.5 text-base text-white outline-1 -outline-offset-1 outline-white/10 placeholder:text-gray-500 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-500 sm:text-sm/6 cursor-pointer";
    let label_class = "block text-sm/6 font-medium text-white";
    let section_title_class = "text-base/7 font-semibold text-white mt-2";
    let section_desc_class = "mt-1 text-sm/6 text-gray-400";
    let button_primary_class = "rounded-md bg-indigo-500 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-400 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 disabled:opacity-50";

    view! {
        // Separator before New Project section
        <div class="border-b border-white/10 pb-12">
            <h2 class=section_title_class>"New Project"</h2>
            <p class=section_desc_class>"Create a new project."</p>

            <ActionForm action=create_project_action>
                <CSRFField />

                <div class="mt-10 grid grid-cols-1 gap-x-6 gap-y-8 sm:grid-cols-6">
                    <div class="sm:col-span-4">
                        <label for="name" class=label_class>
                            "Project Name"
                        </label>
                        <div class="mt-2">
                            <input type="text" name="name" required class=input_class />
                        </div>
                    </div>
                </div>

                <div class="mt-6 flex items-center justify-end gap-x-6">
                    <button type="submit" class=button_primary_class>
                        "Create Project"
                    </button>
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