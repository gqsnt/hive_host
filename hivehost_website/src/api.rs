use crate::app::get_server_url;
use crate::security::permission::{request_server_project_action_front, token_url};
use crate::AppResult;
use common::server_project_action::{ServerProjectAction, ServerProjectActionResponse};
use common::{ProjectSlugStr, StringContent};
use leptos::prelude::Action;

#[cfg(not(feature = "ssr"))]
pub fn fetch_api(
    path: String,
    content: StringContent,
) -> impl std::future::Future<Output = AppResult<ServerProjectActionResponse>> + Send + 'static {
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
            let path_split = path_split[1].split('/').collect::<Vec<_>>();
            let path = path_split[0].to_string();
            format!("{}://{}", path_split[0], path)
        } else {
            path.clone()
        };

        Ok(gloo_net::http::Request::post(&path)
            .header("Access-Control-Allow-Origin", &dns_path)
            .header("Content-Type", "application/json")
            .abort_signal(abort_signal.as_ref())
            .json(&content)?
            .send()
            .await?
            .json::<ServerProjectActionResponse>()
            .await?)
    })
}

#[cfg(feature = "ssr")]
pub async fn fetch_api(
    path: String,
    content: StringContent,
) -> AppResult<ServerProjectActionResponse> {
    let mut headers = reqwest::header::HeaderMap::new();
    let server_vars = crate::ssr::server_vars().expect("SSR server vars missing");
    headers.insert(
        "Access-Control-Allow-Origin",
        format!("http://{}", server_vars.server_url)
            .parse()
            .unwrap(),
    );
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Accept", "application/json".parse().unwrap());
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    Ok(client
        .post(path)
        .json(&content)
        .send()
        .await?
        .json::<ServerProjectActionResponse>()
        .await?)
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::AppResult;
    use common::hosting_action::{HostingAction, HostingActionRequest, HostingActionResponse};
    use common::server_action::{ServerAction, ServerActionResponse};
    use common::server_project_action::{
        ServerProjectAction, ServerProjectActionRequest, ServerProjectActionResponse,
    };
    use common::Slug;
    use secrecy::ExposeSecret;

    pub async fn request_server_project_action_token(
        project_slug: Slug,
        action: ServerProjectAction,
    ) -> AppResult<ServerProjectActionResponse> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        let token = sqlx::types::Uuid::new_v4().to_string();
        let req = ServerProjectActionRequest {
            token: Some(token.clone()),
            action,
            project_slug,
        };
        let _ = client
            .post(format!(
                "http://{}/server_project_action",
                server_vars.server_url
            ))
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .text()
            .await?;
        //log!("request_server_project_action_token response: {}", response);
        Ok(ServerProjectActionResponse::Token(token))
    }

    pub async fn request_hosting_action(
        project_slug: Slug,
        action: HostingAction,
    ) -> AppResult<HostingActionResponse> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        let req = HostingActionRequest {
            action,
            project_slug: project_slug.to_string(),
        };
        Ok(client
            .post(format!("http://{}", server_vars.hosting_url))
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .json::<HostingActionResponse>()
            .await?)
    }

    pub async fn request_server_project_action(
        project_slug: Slug,
        action: ServerProjectAction,
    ) -> AppResult<ServerProjectActionResponse> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        let req = ServerProjectActionRequest {
            token: None,
            action,
            project_slug,
        };
        Ok(client
            .post(format!(
                "http://{}/server_project_action",
                server_vars.server_url
            ))
            .json(&req)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .json::<ServerProjectActionResponse>()
            .await?)
    }

    pub async fn request_server_action(action: ServerAction) -> AppResult<ServerActionResponse> {
        let client = reqwest::Client::new();
        let server_vars = crate::ssr::server_vars()?;
        Ok(client
            .post(format!("http://{}/server_action", server_vars.server_url))
            .json(&action)
            .bearer_auth(server_vars.token_action_auth.expose_secret())
            .send()
            .await?
            .json::<ServerActionResponse>()
            .await?)
    }
}

pub type ServerProjectActionFront = Action<
    (
        ProjectSlugStr,
        ServerProjectAction,
        Option<String>,
        Option<String>,
    ),
    AppResult<ServerProjectActionResponse>,
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
    content: Option<String>,
    csrf: Option<String>,
) -> AppResult<ServerProjectActionResponse> {
    let response = request_server_project_action_front(project_slug, action, csrf).await?;
    if let ServerProjectActionResponse::Token(token) = response.clone() {
        Ok(fetch_api(
            token_url(&get_server_url().await?, &token),
            StringContent { inner: content },
        )
        .await?)
    } else {
        Ok(response)
    }
}
