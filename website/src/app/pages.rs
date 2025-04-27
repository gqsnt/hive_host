use leptos::context::provide_context;
use leptos::prelude::{ClassAttribute, Get, OnceResource, ServerFnError, Signal};
use leptos::prelude::codee::{Decoder, Encoder};
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use common::{ProjectSlug, UserSlug};
use crate::app::components::csrf_field::generate_csrf;
use crate::models::{Project, User};

pub mod home;
pub mod login;
pub mod signup;
pub mod user;


#[derive(Default, Clone, Debug, Store)]
pub struct GlobalState {
    pub csrf: Option<String>,
    pub user:Option<(UserSlug, User)>,
    pub project:Option<(ProjectSlug, Project)>,
    pub hosting_url:Option<String>,
}
