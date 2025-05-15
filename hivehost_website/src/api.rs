use crate::security::permission::{request_server_project_action_front, request_token_action_front};
use crate::{AppError, AppResult};
use common::server_action::project_action::{ProjectAction, ProjectResponse};
use common::ProjectSlugStr;
use leptos::logging::log;
use leptos::prelude::Action;

use common::server_action::token_action::{TokenAction, UsedTokenActionResponse};
use web_sys::FormData;

#[cfg(feature = "hydrate")]
pub fn fetch_api(
    path:String,
    body: Option<FormData>,
) -> impl std::future::Future<Output = Option<UsedTokenActionResponse>> + Send + 'static {
    use leptos::logging::log;
    use leptos::prelude::on_cleanup;
    use send_wrapper::SendWrapper;



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
    _content: Option<FormData>,
) -> impl std::future::Future<Output = Option<UsedTokenActionResponse>> + Send + 'static{
    log!("api front request error: {path}");
    async {
        None
    }
}




#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::{AppError, AppResult};
    use common::server_action::project_action::{
        ProjectAction, ProjectResponse,
    };
    use common::server_action::user_action::{ServerUserAction, ServerUserResponse};
    use common::{ServerId, Slug};
    use common::tarpc_client::TarpcClientError;
    use crate::ssr::WsClients;

    pub async fn request_server_project_action(
        server_id: i64,
        project_slug: Slug,
        action: ProjectAction,
        client:Option<WsClients>,
    ) -> AppResult<ProjectResponse> {
        let handle_client = |client:WsClients|async move {
            match client.get(&server_id){
                None => Err(AppError::TrpcClientError(TarpcClientError::NotConnected)),
                Some(client) => {
                    client.project_action( project_slug.to_string(), action).await.map_err(Into::into)
                }
            }
        };
        
        match client{
            Some(client) => {
                handle_client(client).await
            },
            None =>{
                let client = crate::ssr::ws_clients()?;
                handle_client(client).await
            } 
        }
        
    }

    pub async fn request_user_action(server_id: ServerId,action: ServerUserAction) -> AppResult<ServerUserResponse> {
        match crate::ssr::ws_clients()?.get(&server_id){
            None => Err(AppError::TrpcClientError(TarpcClientError::NotConnected)),
            Some(client) => {
                client.user_action(action).await.map_err(Into::into)
            }
        }
    }
}

pub type ServerProjectActionFront = Action<
    (
        i64,
        ProjectSlugStr,
        ProjectAction,
        Option<String>,
    ),
    AppResult<ProjectResponse>,
>;

pub fn get_action_server_project_action() -> ServerProjectActionFront {
    Action::new(
        |input: &(
            i64,
            ProjectSlugStr,
            ProjectAction,
            Option<String>,
        )| {
            let (server_id,project_slug, action, csrf) = input.clone();
            async move {
                request_server_project_action_front(server_id,project_slug, action, csrf)
                    .await
            }
        },
    )
}


pub async fn get_action_token_action(
    server_id: i64,
    project_slug: ProjectSlugStr,
    action: TokenAction,
    csrf: Option<String>,
    form:Option<FormData>,
) -> AppResult<UsedTokenActionResponse> {
    log!("get_action_token_action: {project_slug} {action:?} {csrf:?}");
    let token_url = request_token_action_front(server_id, project_slug, action, csrf).await?;
    fetch_api(token_url, form).await.ok_or(AppError::Custom("Error fetching token action".to_string()))
}


