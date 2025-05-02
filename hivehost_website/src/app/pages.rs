use crate::models::{Project, User};
use common::permission::Permission;
use common::Slug;
use reactive_stores::Store;

pub mod home;
pub mod login;
pub mod signup;
pub mod user;

#[derive(Default, Clone, Debug, Store)]
pub struct GlobalState {
    pub csrf: Option<String>,
    pub user: Option<(Slug, User)>,
    pub project: Option<(Slug, Permission, Project)>,
    pub hosting_url: Option<String>,
}
