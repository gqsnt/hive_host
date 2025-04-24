use leptos::context::provide_context;
use leptos::prelude::{Get, OnceResource, Signal};
use serde::{Deserialize, Serialize};

pub mod home;
pub mod login;
pub mod signup;
pub mod user;


#[derive(Default, Deserialize, Clone, Debug, Serialize)]
pub struct CsrfValue(pub String);

pub fn include_csrf() {
    let csrf = OnceResource::new_blocking(
        async {
        crate::app::components::csrf_field::generate_csrf().await
});
    let csrf_signal = Signal::derive(move || {
        CsrfValue(csrf.get()
            .map(|csrf| csrf.unwrap_or_default())
            .unwrap_or_default())
    });
    provide_context(csrf_signal);
}