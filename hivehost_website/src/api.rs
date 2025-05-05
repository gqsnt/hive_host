use crate::security::permission::{request_server_project_action_front};
use crate::AppResult;
use common::website_to_server::server_project_action::{ServerProjectAction, ServerProjectResponse};
use common::{ProjectSlugStr};
use leptos::prelude::Action;

#[cfg(feature = "ssr")]
pub mod ssr {
    use tarpc::context;
    use crate::{AppResult};
    use common::website_to_server::server_action::{ServerAction, ServerActionResponse};
    use common::website_to_server::server_project_action::{
        ServerProjectAction, ServerProjectResponse,
    };
    use common::Slug;
    use common::hosting::{HostingAction, HostingResponse};

    pub async fn request_hosting_action(
        project_slug: Slug,
        action: HostingAction,
    ) -> AppResult<HostingResponse> {
        let ws_client = crate::ssr::ws_client()?;
        ws_client.hosting_action(
            context::current(),project_slug.to_string(), action,
        ).await.map_err(Into::into)
    }

    pub async fn request_server_project_action(
        project_slug: Slug,
        action: ServerProjectAction,
    ) -> AppResult<ServerProjectResponse> {
        let ws_client = crate::ssr::ws_client()?;
        ws_client.server_project_action(
            context::current(),project_slug.to_string(), action,
        ).await.map_err(Into::into)
    }

    pub async fn request_server_action(action: ServerAction) -> AppResult<ServerActionResponse> {
        let ws_client = crate::ssr::ws_client()?;
        ws_client.server_action(context::current(),action).await.map_err(Into::into)
    }
}

pub type ServerProjectActionFront = Action<
    (
        ProjectSlugStr,
        ServerProjectAction,
        Option<String>,
    ),
    AppResult<ServerProjectResponse>,
>;

pub fn get_action_server_project_action() -> ServerProjectActionFront {
    Action::new(
        |input: &(
            ProjectSlugStr,
            ServerProjectAction,
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
    action: ServerProjectAction,
    csrf: Option<String>,
) -> AppResult<ServerProjectResponse> {
    let response = request_server_project_action_front(project_slug, action, csrf).await?;
    if let ServerProjectResponse::Token(_token) = response.clone() {
        Ok(ServerProjectResponse::Ok)
    } else {
        Ok(response)
    }
}
