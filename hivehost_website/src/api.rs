use crate::security::permission::{request_server_project_action_front};
use crate::AppResult;
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::{ProjectSlugStr};
use leptos::prelude::Action;

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::{AppResult};
    use common::server_action::project_action::{
        ProjectAction, ProjectResponse,
    };
    use common::Slug;
    use common::server_action::user_action::{ServerUserAction, ServerUserResponse};
    

    pub async fn request_server_project_action(
        project_slug: Slug,
        action: ProjectAction,
    ) -> AppResult<ProjectResponse> {
        let ws_client = crate::ssr::ws_client()?;
        ws_client.project_action( project_slug.to_string(), action).await.map_err(Into::into)
    }

    pub async fn request_user_action(action: ServerUserAction) -> AppResult<ServerUserResponse> {
        let ws_client = crate::ssr::ws_client()?;
        ws_client.user_action(action).await.map_err(Into::into)
    }
}

pub type ServerProjectActionFront = Action<
    (
        ProjectSlugStr,
        ProjectAction,
        Option<String>,
    ),
    AppResult<ProjectResponse>,
>;

pub fn get_action_server_project_action() -> ServerProjectActionFront {
    Action::new(
        |input: &(
            ProjectSlugStr,
            ProjectAction,
            Option<String>,
        )| {
            let (project_slug, action, csrf) = input.clone();
            async move {
                get_action_server_project_action_inner(project_slug, action, csrf)
                    .await
            }
        },
    )
}

pub async fn get_action_server_project_action_inner(
    project_slug: ProjectSlugStr,
    action: ProjectAction,
    csrf: Option<String>,
) -> AppResult<ProjectResponse> {
    let response = request_server_project_action_front(project_slug, action, csrf).await?;
    if let ProjectResponse::Token(_token) = response.clone() {
        Ok(ProjectResponse::Ok)
    } else {
        Ok(response)
    }
}
