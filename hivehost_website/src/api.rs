use crate::security::permission::{request_server_project_action_front};
use crate::AppResult;
use common::website_to_server::server_project_action::{ServerProjectAction, ServerProjectResponse};
use common::{ProjectSlugStr};
use leptos::prelude::Action;

#[cfg(feature = "ssr")]
pub mod ssr {
    use leptos::logging::log;
    use crate::{AppError, AppResult};
    use common::website_to_server::server_action::{ServerAction, ServerActionResponse};
    use common::website_to_server::server_project_action::{
        ServerProjectAction, ServerProjectResponse,
    };
    use common::Slug;
    use common::hosting::{HostingAction, HostingResponse};
    use common::website_to_server::{WebSiteToServerAction, WebSiteToServerResponse};

    pub async fn request_hosting_action(
        project_slug: Slug,
        action: HostingAction,
    ) -> AppResult<HostingResponse> {
        let mc = crate::ssr::multiplexer_client()?;
        let response = mc.send(
            WebSiteToServerAction::from_hosting_action(project_slug.to_string(), action),
        ).await;
        match response {
            Ok(WebSiteToServerResponse::HostingActionResponse(action_response)) => {
                Ok(action_response)
            }
            _ => Err(AppError::Custom("server action failed".to_string())),
        }
    }

    pub async fn request_server_project_action(
        project_slug: Slug,
        action: ServerProjectAction,
    ) -> AppResult<ServerProjectResponse> {
        let mc = crate::ssr::multiplexer_client()?;
        let response = mc.send(
            WebSiteToServerAction::from_server_project_action(project_slug.to_string(), action),
        ).await;
        match response {
            Ok(WebSiteToServerResponse::ServerProjectActionResponse(action_response)) => {
                Ok(action_response)
            }
            _ => Err(AppError::Custom("server action failed".to_string())),
        }
    }

    pub async fn request_server_action(action: ServerAction) -> AppResult<ServerActionResponse> {
        let mc = crate::ssr::multiplexer_client()?;
        let response = mc.send(action.into()).await;
        match response {
            Ok(WebSiteToServerResponse::ServerActionResponse(action_response)) => {
                Ok(action_response)
            }
            Err(e) => {
                log!("Client Error: {:?}", e);
                Err(AppError::Custom("server action failed".to_string()))
            }
            Ok(WebSiteToServerResponse::HostingActionResponse(r)) => {
                log!("Hosting Action Response inside Server: {:?}", r);
                Err(AppError::Custom("server action failed".to_string()))
            }
            Ok(WebSiteToServerResponse::ServerProjectActionResponse(r)) => {
                log!("Server Project Action inside Server: {:?}", r);
                Err(AppError::Custom("server action failed".to_string()))
            }
            Ok(WebSiteToServerResponse::Pong) => {
                log!("Pong");
                 Ok(ServerActionResponse::Ok)
            }
            Ok(WebSiteToServerResponse::Error(e)) => {
                log!("Response Error: {:?}", e);
                Err(AppError::Custom("server action failed".to_string()))
            }
        }
    }
}

pub type ServerProjectActionFront = Action<
    (
        ProjectSlugStr,
        ServerProjectAction,
        Option<String>,
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
            Option<String>,
        )| {
            let (project_slug, action, string_content, csrf) = input.clone();
            async move {
                get_action_server_project_action_inner(project_slug, action, string_content, csrf)
                    .await
            }
        },
    )
}

pub async fn get_action_server_project_action_inner(
    project_slug: ProjectSlugStr,
    action: ServerProjectAction,
    _content: Option<String>,
    csrf: Option<String>,
) -> AppResult<ServerProjectResponse> {
    let response = request_server_project_action_front(project_slug, action, csrf).await?;
    if let ServerProjectResponse::Token(_token) = response.clone() {
        Ok(ServerProjectResponse::Ok)
    } else {
        Ok(response)
    }
}
