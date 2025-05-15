use serde::{Deserialize, Serialize};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct GithubRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub private:bool,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubBranch{
    pub name:String,
    pub commit:String,
}

impl From<GithubBranchApi> for GithubBranch{
    fn from(value: GithubBranchApi) -> Self {
        Self {
            name: value.name,
            commit: value.commit.sha,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubBranchApi{
    pub name:String,
    pub commit:GithubBranchCommitApi,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubBranchCommitApi{
    pub sha:String,
}




#[cfg(feature = "ssr")]
pub mod ssr{
    use std::ops::{Add, Sub, SubAssign};
    use axum::body::to_bytes;
    use axum::extract::{FromRequest, Query, Request, State};
    use axum::Json;
    use axum::response::{IntoResponse, Redirect};
    use chrono::TimeDelta;
    use http::StatusCode;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use leptos::either::EitherOf14::N;
    use leptos::leptos_dom::log;
    use leptos_axum::redirect;
    use octocrab::models::webhook_events::{EventInstallation, WebhookEvent, WebhookEventPayload, WebhookEventType};
    use octocrab::models::webhook_events::payload::InstallationWebhookEventAction;
    use reqwest::{Client, ClientBuilder};
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use common::{Slug, GITHUB_APP_NAME};
    use crate::app::pages::user::projects::project::project_settings::server_fns::ssr::handle_auto_deploy_git;
    use crate::AppResult;
    use crate::github::GithubRepo;
    use crate::models::User;
    use crate::security::ssr::AppAuthSession;
    use crate::ssr::{AppState, ServerVars};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct InstallReposResponse {
        pub total_count:usize,
        pub repositories: Vec<GithubRepo>,
    }



    #[derive(Debug, Serialize, Deserialize)]
    pub struct GithubJwt {
        iat: usize,
        exp: usize,
        iss: String,
        alg: String,
    }
    
    impl GithubJwt {
        pub fn new(
            iss:String
        ) -> Self {
            let now = chrono::Utc::now();
            let delta = TimeDelta::seconds(60);
            Self {
                iat: now.sub(delta).timestamp() as usize,
                exp: now.add(delta).timestamp() as usize,
                iss,
                alg: "RS256".to_string(),
            }
        }
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct GithubAppToken {
        pub token: String,
    }




    
    pub fn get_git_client(
    )->Client{
        let mut headers=  reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(GITHUB_APP_NAME),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            reqwest::header::HeaderValue::from_static("2022-11-28"),
        );
        reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build().unwrap()
    }
    
    
    pub async  fn get_authenticated_git_client(
         server_vars:&ServerVars,
         installation_id:i64,
    ) -> AppResult<(String, Client)>{
        let client = get_git_client();
        let token = encode(
            &Header::new(Algorithm::RS256),
            &GithubJwt::new(server_vars.github_client_id.to_string()),
            &EncodingKey::from_rsa_pem(&server_vars.git_pem).unwrap()
        ).unwrap();
        let token:GithubAppToken = client
            .post(format!("https://api.github.com/app/installations/{}/access_tokens", installation_id))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?
            .json().await?;
        Ok((token.token, client))
    }
    
    pub async  fn get_all_repos(client: Client, token:String) -> AppResult<Vec<GithubRepo>>{
        let mut repos = vec![];
        let mut page = 1;
        let per_page= 100;
        let mut total = None;
        loop {
            let response: InstallReposResponse = client
                .get(format!("https://api.github.com/installation/repositories?per_page={per_page}&page={page}"))
                .header(reqwest::header::AUTHORIZATION, format!("token {token}"))
                .send()
                .await?
                .json().await?;
            if total.is_none(){
                total = Some(response.total_count);
            }
            repos.extend(response.repositories);
            if let Some(total_count) = total{
                if repos.len() >= total_count{
                    break;
                }
            }
            page += 1;
        }
        repos.reverse();
        Ok(repos)
    }
   


    #[derive( Debug,  Deserialize)]
    pub struct GithubSetupCallbackQuery {
        pub installation_id: i64,
        pub setup_action: String,
    }
    
    
    
    
    pub async fn github_post_install_callback(
        State(app_state): State<AppState>,
        auth:AppAuthSession,
        Query(query): Query<GithubSetupCallbackQuery>,
        request: Request,
    ) -> impl IntoResponse {
        match auth.current_user{
            None => {
                Redirect::to("/login")
            }
            Some(user) => {
                let mut total_time = 0.0;
                while !app_state.github_install_cache.contains_key(&query.installation_id) && total_time < 10.0{
                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                    total_time += 0.25;
                }
                match app_state.github_install_cache.get(&query.installation_id).await{
                    None => {
                        Redirect::to("/")
                    }
                    Some((login,avatar_url, html_url)) => {
                        let pool = app_state.pool.clone();
                        sqlx::query!(
                            r#"insert into user_githubs (installation_id, user_id, login, avatar_url, html_url) values ($1, $2,$3,$4,$5)"#,
                            query.installation_id,
                            user.id,
                            login,
                            avatar_url,
                            html_url
                            )
                            .execute(&pool)
                            .await
                            .unwrap();
                        Redirect::to("/user/settings")
                    }
                }
            }
        }
    }
    

    pub async fn github_webhook(
        State(app_state): State<AppState>,
        request: Request,
    ) -> impl IntoResponse {
        let (parts, body) = request.into_parts();
        let body = to_bytes(body, usize::MAX).await.unwrap();
        let header = parts.headers.get("X-GitHub-Event").unwrap().to_str().unwrap();
        let event = WebhookEvent::try_from_header_and_body(header, &body).unwrap();
        match event.kind {
            WebhookEventType::Push => {
                if let Some(repo) = event.repository{
                    if let WebhookEventPayload::Push(payload) = event.specific{
                        let branch_name = match payload.r#ref.split_once('/'){
                            None => {
                                log!("Received push event with no branch name");
                                return (StatusCode::OK, "".to_string());
                            }
                            Some((before, after)) => {
                                let (ref_type, branch_name) = after.split_once('/').unwrap();
                                if ref_type != "heads"{
                                    log!("Received push event with no branch name");
                                    return (StatusCode::OK, "".to_string());
                                }
                                branch_name.to_string()
                            }
                        };
                        let commit = payload.after;
                        let repo_full_name = repo.full_name.unwrap_or_default();
                        log!("Received push event for repo {} with branch {} and commit {}", repo_full_name, branch_name, commit);
                        let pool = app_state.pool.clone();
                        let git_projects_auto_deploy = sqlx::query!(
                            r#"update projects_github set last_commit = $1 where repo_full_name = $2 and branch_name = $3 and auto_deploy = true returning id, dev_commit"#,
                             commit.clone(),
                            repo_full_name,
                            branch_name
                        )
                            .fetch_all(&pool)
                            .await
                            .unwrap()
                            .into_iter()
                            .map(|row|(row.id, row.dev_commit))
                            .collect::<Vec<_>>();
                         let _update_no_auto_deploy_result = sqlx::query!(
                            r#"update projects_github set last_commit = $1 where repo_full_name = $2 and branch_name = $3 and auto_deploy = false"#,
                            commit.clone(),
                            repo_full_name,
                            branch_name
                        )
                            .execute(&pool)
                            .await
                            .unwrap();
                        for (git_project_id, git_dev_commit) in git_projects_auto_deploy{
                            log!("Found git project {} with branch {} and commit {} to audodeploy", repo_full_name, branch_name, commit);
                            let project = sqlx::query!(
                                r#"select * from projects where project_github_id = $1"#,
                                git_project_id
                            )
                                .fetch_one(&pool)
                                .await
                                .unwrap();
                            let last_snapshot = sqlx::query!(
                                r#"select git_commit from projects_snapshots where project_id = $1 order by created_at desc limit 1"#,
                                project.id
                            )
                                .fetch_optional(&pool)
                                .await
                                .unwrap();
                            match handle_auto_deploy_git(
                                &pool,
                                app_state.ws_clients.clone(),
                                project.server_id,
                                Slug::new(project.id, project.name),
                                git_project_id,
                                branch_name.clone(),
                                git_dev_commit,
                                last_snapshot.and_then(|ls|ls.git_commit),
                                commit.clone(),
                            ).await{
                                Ok(_) => {
                                    log!("Auto deploy git project {} with commit {}", repo_full_name, commit);
                                }
                                Err(e) => {
                                    log!("Error auto deploying git project {} with commit {}: {}", repo_full_name, commit, e);
                                }
                            }
                        }
                    }
                }
            },
            WebhookEventType::Installation => {
                if let Some(EventInstallation::Full(installation)) = event.installation{
                    if let  WebhookEventPayload::Installation(specific) = event.specific{
                        match specific.action{
                            InstallationWebhookEventAction::Created => {
                                app_state.github_install_cache.insert(
                                    installation.id.0.cast_signed(),
                                    (installation.account.login, installation.account.avatar_url.to_string(), installation.account.html_url.to_string()),
                                ).await;
                            }
                            InstallationWebhookEventAction::Deleted => {
                                let pool = app_state.pool.clone();
                                sqlx::query!(
                                    r#"delete from user_githubs where installation_id = $1"#,
                                    installation.id.0.cast_signed()
                                ).execute(&pool)
                                    .await.unwrap();
                            }
                            InstallationWebhookEventAction::NewPermissionsAccepted => {}
                            InstallationWebhookEventAction::Suspend => {
                                let pool = app_state.pool.clone();
                                sqlx::query!(
                                    r#"update user_githubs set suspended = true where installation_id = $1"#,
                                    installation.id.0.cast_signed()
                                ).execute(&pool)
                                    .await.unwrap();
                            }
                            InstallationWebhookEventAction::Unsuspend => {
                                let pool = app_state.pool.clone();
                                sqlx::query!(
                                    r#"update user_githubs set suspended = false where installation_id = $1"#,
                                    installation.id.0.cast_signed()
                                ).execute(&pool)
                                    .await.unwrap();
                            }
                            _ => {}
                        }
                    }
                }
                
            },
            _ => log!("Received not handled webhook event {:?}", event.kind),
        };
        (StatusCode::OK, "".to_string())
    }
}
