use leptos::{component, server, view, IntoView};
use leptos::either::Either;
use leptos::prelude::{Get, Resource, ServerFnError, Transition};
use leptos::prelude::ElementChild;

#[component]
pub fn CSRFField() -> impl IntoView {
    let csrf_resource = Resource::new(|| (), |_| generate_csrf());

    view! {
        <Transition fallback=|| {
            view! { <p>"Loading..."</p> }
        }>
            {move || {
                csrf_resource
                    .get()
                    .map(|n| match n {
                        Err(e) => {
                            Either::Left(
                                view! {
                                    {format!(
                                        "Page Load Failed: {e}. Please reload the page or try again later.",
                                    )}
                                },
                            )
                        }
                        Ok(csrf_hash) => {
                            Either::Right(
                                view! { <input type="hidden" name="csrf" value=csrf_hash /> },
                            )
                        }
                    })
            }}
        </Transition>
    }
}



#[server]
async fn generate_csrf() -> Result<String, ServerFnError> {
    use crate::security::utils::ssr::gen_easy_hash;

    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    let auth_session = auth.session.get_session_id().to_string();
    Ok(gen_easy_hash(
        auth_session,
        server_vars.csrf_server.to_secret(),
    ))
}
