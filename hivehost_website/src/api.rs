use std::time::Duration;
use leptos::logging::log;
use crate::security::permission::{request_server_project_action_front, request_token_action_front, token_url};
use crate::{AppError, AppResult};
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::{ProjectSlugStr};
use leptos::prelude::Action;

use web_sys::{Blob, FormData};
use web_sys::js_sys::Array;
use common::server_action::token_action::{TokenAction, TokenActionResponse, UsedTokenActionResponse};
use crate::app::get_server_url_front;

#[cfg(feature = "hydrate")]
pub fn fetch_api(
    path:String,
    body: Option<FormData>,
) -> impl std::future::Future<Output = Option<UsedTokenActionResponse>> + Send + 'static {
    use leptos::logging::log;
    use leptos::prelude::on_cleanup;
    use send_wrapper::SendWrapper;
    use wasm_bindgen::JsValue;



    SendWrapper::new(async move {
        let abort_controller = SendWrapper::new(web_sys::AbortController::new().ok());
        let abort_signal = abort_controller.as_ref().map(|a| a.signal());

        //abort in-flight requests if, e.g., we've navigated away from this page
        on_cleanup(move || {
            if let Some(abort_controller) = abort_controller.take() {
                abort_controller.abort()
            }
        });


        let path_split = path.split("://").collect::<Vec<_>>();
        let dns_path = if path_split.len() > 1 {
            let path_resplit = path_split[1].split('/').collect::<Vec<_>>();
            format!("{}://{}", path_split[0], path_resplit[0])
        } else {
            path.clone()
        };
        let body = body.unwrap_or({
            FormData::new().unwrap()
        });
        
        gloo_net::http::Request::post(&path)
            .header("Access-Control-Allow-Origin", &dns_path)
            .abort_signal(abort_signal.as_ref())
            .body(body)
            .unwrap()
            .send()
            .await
            .map_err(|e| log!("api front request error: {e}"))
            .ok()?
            .json::<UsedTokenActionResponse>()
            .await
            .map_err(|e| log!("api front response error: {e}"))
            .ok()
    })
}

#[cfg(feature = "ssr")]
pub fn fetch_api(
    path: String,
    content: Option<FormData>,
) -> impl std::future::Future<Output = Option<UsedTokenActionResponse>> + Send + 'static{
    log!("api front request error: {path}");
    async {
        None
    }
}




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
                request_server_project_action_front(project_slug, action, csrf)
                    .await
            }
        },
    )
}


pub async fn get_action_token_action(
    project_slug: ProjectSlugStr,
    action: TokenAction,
    csrf: Option<String>,
    form:Option<FormData>,
) -> AppResult<UsedTokenActionResponse> {
    log!("get_action_token_action: {project_slug} {action:?} {csrf:?}");
    let token = request_token_action_front(project_slug, action, csrf).await?;
    fetch_api(token_url(&get_server_url_front().await?, &token), form).await.ok_or(AppError::Custom("Error fetching token action".to_string()))
}


