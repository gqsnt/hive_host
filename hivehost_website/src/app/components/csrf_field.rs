use crate::app::pages::{GlobalState, GlobalStateStoreFields};
use crate::AppResult;
use leptos::prelude::Get;
use leptos::prelude::{expect_context, Signal};
use leptos::server_fn::codec::Bincode;
use leptos::{component, server, view, IntoView};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

pub type CsrfSignal = Signal<Option<CsrfValue>>;

#[component]
pub fn CSRFField() -> impl IntoView {
    let global_state: Store<GlobalState> = expect_context();
    view! {
        <input
            type="hidden"
            name="csrf"
            value=move || global_state.csrf().get().unwrap_or_default()
        />
    }
}

#[server(input=Bincode, output=Bincode)]
pub async fn generate_csrf() -> AppResult<String> {
    use crate::security::utils::ssr::gen_easy_hash;

    let auth = crate::ssr::auth(true)?;
    let server_vars = crate::ssr::server_vars()?;
    let auth_session = auth.session.get_session_id().to_string();
    Ok(gen_easy_hash(
        auth_session,
        server_vars.csrf_server.to_secret(),
    ))
}

#[derive(Default, Serialize, Clone, Debug, Deserialize)]
pub struct CsrfValue(pub String);
