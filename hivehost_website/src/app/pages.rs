use crate::models::{Project, User};
use common::{ProjectSlug, UserSlug};
use reactive_stores::Store;

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
