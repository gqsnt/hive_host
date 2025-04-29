use common::{ProjectSlugStr, UserSlugStr};


pub fn project_path(project_slug: ProjectSlugStr) -> String {
    format!("/projects/{}", project_slug)
}

pub fn user_project_path(user_slug: UserSlugStr, project_slug: ProjectSlugStr) -> String {
    format!("{}/{}", user_projects_path(user_slug), project_slug)
}
pub fn user_projects_path(user_slug: UserSlugStr) -> String {
    format!("{}/projects", user_path(user_slug))
}

pub fn user_path(user_slug: UserSlugStr) -> String {
    format!("/sftp/users/{}", user_slug)
}

