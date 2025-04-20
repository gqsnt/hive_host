use leptos::logging::log;
use serde::de::DeserializeOwned;
use serde::Serialize;
use secrecy::ExposeSecret;
use leptos::prelude::{Action, ServerFnError};
use leptos::server;
use common::ProjectSlugStr;
use common::server_action::ServerActionResponse;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use crate::security::permission::{request_server_project_action_front, token_url};

#[cfg(not(feature = "ssr"))]
pub fn fetch_api<T>(
    path: &str,
) -> impl std::future::Future<Output = Option<T>> + Send + '_
where
    T: Serialize + DeserializeOwned,
{
    use leptos::prelude::on_cleanup;
    use send_wrapper::SendWrapper;
    use leptos::logging::log;

    SendWrapper::new(async move {
        let abort_controller =
            SendWrapper::new(web_sys::AbortController::new().ok());
        let abort_signal = abort_controller.as_ref().map(|a| a.signal());

        // abort in-flight requests if, e.g., we've navigated away from this page
        on_cleanup(move || {
            if let Some(abort_controller) = abort_controller.take() {
                abort_controller.abort()
            }
        });

        gloo_net::http::Request::post(path)
            .header("Access-Control-Allow-Origin", "http://127.0.0.1:3002")
            .abort_signal(abort_signal.as_ref())
            .send()
            .await
            .map_err(|e| log!("{e}"))
            .ok()?
            .json()
            .await
            .map_err(|e| log!("{e}"))
            .ok()
    })
}

#[cfg(feature = "ssr")]
pub async fn fetch_api<T>(path: &str) -> Option<T>
where
    T: Serialize + DeserializeOwned,
{
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Access-Control-Allow-Origin", "http://127.0.0.1:3002".parse().unwrap());
    let client = reqwest::Client::builder().default_headers(headers).build().unwrap();
    client.post(path)
        .send()
        .await
        .map_err(|e| log!("{e}"))
        .ok()?
        .json()
        .await
        .map_err(|e| log!("{e}"))
        .ok()
}


#[cfg(feature = "ssr")]
pub mod ssr{
    use leptos::logging::log;
    use leptos::prelude::ServerFnError;
    use secrecy::ExposeSecret;
    use common::ProjectSlug;
    use common::server_action::{ServerAction, ServerActionResponse};
    use common::server_project_action::{ServerProjectAction, ServerProjectActionRequest, ServerProjectActionResponse};


    pub async fn request_server_project_action_token(
        project_slug: ProjectSlug,
        action:ServerProjectAction,
    ) -> Result<ServerProjectActionResponse, ServerFnError> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        let token = sqlx::types::Uuid::new_v4().to_string();
        let req = ServerProjectActionRequest {
            token: Some(token.clone()),
            action,
            project_slug,
        };
        let _ = client
            .post("http://127.0.0.1:3002/server_project_action")
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .json::<ServerProjectActionResponse>()
            .await?;
        Ok(ServerProjectActionResponse::Token(token))
    }
    
    
    
    pub async fn request_server_project_action(
        project_slug: ProjectSlug,
        action:ServerProjectAction,
    ) -> Result<ServerProjectActionResponse, ServerFnError> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        let req = ServerProjectActionRequest {
            token: None,
            action,
            project_slug,
        };
        let response = client
            .post("http://127.0.0.1:3002/server_project_action")
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .text()
            .await?;
        let response = serde_json::from_str::<ServerProjectActionResponse>(&response)
            .map_err(|e| {
                ServerFnError::new(e.to_string())
            })?;
        
        Ok(response)
    }
    
    

    pub async fn request_server_action(
        action:ServerAction,
    ) -> Result<ServerActionResponse, ServerFnError> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        Ok(client.post(
            "http://127.0.0.1:3002/server_action"
        )
            .json(&action)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .json::<ServerActionResponse>().await?)
    }
}


pub type ServerProjectActionFront = Action<(ProjectSlugStr, ServerProjectAction), Result<ServerProjectActionResponse, ServerFnError>>;

pub fn get_action_server_project_action(
) -> ServerProjectActionFront{
    Action::new(|input: &(ProjectSlugStr, ServerProjectAction)| {
        let (project_slug, action) = input.clone();
        async move {
            get_action_server_project_action_inner(project_slug, action).await
        }
    })
}



pub async fn get_action_server_project_action_inner(
    project_slug:ProjectSlugStr,
    action:ServerProjectAction
)->Result<ServerProjectActionResponse, ServerFnError>{
    if let Ok(r) = request_server_project_action_front(project_slug, action).await{
        return if let ServerProjectActionResponse::Token(token) = r.clone(){
            match fetch_api::<ServerProjectActionResponse>(token_url(token).as_str()).await{
                None => {
                    return Err(ServerFnError::new("Error fetching token response"));
                }
                Some(r) => {
                    Ok(r)
                }
            }
        }else{
            Ok(r)
        }
    }
    Err(ServerFnError::new("Error"))
}