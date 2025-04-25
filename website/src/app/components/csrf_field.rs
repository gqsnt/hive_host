use crate::app::pages::CsrfValue;
use leptos::prelude::ServerFnError;
use leptos::prelude::{expect_context, Signal, Suspend, Suspense};
use leptos::{component, server, view, IntoView};

#[component]
pub fn CSRFField() -> impl IntoView {
    let csrf_value = expect_context::<Signal<CsrfValue>>();

    view! {
        <Suspense fallback=move || {
            view! { <div></div> }
        }>
            {move || Suspend::new(async move {
                view! { <input type="hidden" name="csrf" value=csrf_value().0 /> }
            })}
        </Suspense>
    }
}

#[server]
pub async fn generate_csrf() -> Result<String, ServerFnError> {
    use crate::security::utils::ssr::gen_easy_hash;

    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    let auth_session = auth.session.get_session_id().to_string();
    Ok(gen_easy_hash(
        auth_session,
        server_vars.csrf_server.to_secret(),
    ))
}
