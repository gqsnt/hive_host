use leptos::prelude::{Get};
use leptos::prelude::{ServerFnError};
use leptos::prelude::{expect_context, Signal, Suspense};
use leptos::{component, server, view, IntoView};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use crate::app::pages::{GlobalState, GlobalStateStoreFields};

pub type CsrfSignal = Signal<Option<CsrfValue>>;

#[component]
pub fn CSRFField() -> impl IntoView {
    let global_state:Store<GlobalState> = expect_context();
    view! {
        <input
            type="hidden"
            name="csrf"
            value=move || global_state.csrf().get().unwrap_or_default()
        />
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



#[derive(Default, Deserialize, Clone, Debug, Serialize)]
pub struct CsrfValue(pub String);

