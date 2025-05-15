use crate::models::{Project, User};
use common::server_action::permission::Permission;
use common::Slug;
use reactive_stores::{ Store};


pub mod home;
pub mod login;
pub mod signup;
pub mod user;

#[derive(Default, Clone, Debug, Store)]
pub struct GlobalState {
    pub csrf: Option<String>,
    pub user: Option<(Slug, User)>,
    pub project_state: Option<ProjectState>,
}



#[derive(Clone, Debug, Store)]
pub struct ProjectState{
    pub slug: Slug,
    pub permission: Permission,
    pub project: Project,
}

impl ProjectState {
    pub fn new(slug: Slug, permission: Permission, project: Project) -> Self {
        Self {
            slug,
            permission,
            project,
        }
    }
    
}