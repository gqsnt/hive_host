

pub mod project_action;

pub mod permission;
pub mod user_action;
pub mod token_action;

#[cfg(feature = "tarpc-website-to-server")]
pub mod tarpc{
    use tarpc::context;
    use tarpc::client::RpcError;
    use crate::ProjectSlugStr;
    use crate::server_action::project_action::{ProjectAction, ProjectResponse};
    use crate::server_action::token_action::{TokenAction, TokenActionResponse};
    use crate::server_action::user_action::{ServerUserAction, ServerUserResponse};
    use crate::tarpc_client::{TarpcClient, TarpcClientError};

    #[tarpc::service]
    pub trait WebsiteToServer {
        
        async fn token_action(project_slug_str: ProjectSlugStr, action:TokenAction) -> TokenActionResponse;
        
        async fn user_action(action: ServerUserAction) -> ServerUserResponse;
        async fn project_action(project_slug: ProjectSlugStr, action: ProjectAction) -> ProjectResponse;
    }

    impl TarpcClient<WebsiteToServerClient>{
        
        pub async fn token_action(
            &self,
            project_slug_str:ProjectSlugStr,
            action: TokenAction
        ) -> Result<TokenActionResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client.token_action(context::current(),project_slug_str.clone(), action.clone()).await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client.token_action(context::current(),project_slug_str, action).await
                    .map_err(From::from)
            } else {
                result.map_err(From::from)
            }
        }
        
        
        pub async fn project_action(
            &self,
            project_slug:ProjectSlugStr,
            action: ProjectAction
        ) -> Result<ProjectResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client.project_action(context::current(),project_slug.clone(), action.clone()).await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client.project_action(context::current(),project_slug, action).await
                    .map_err(From::from)
            } else {
                result.map_err(From::from)
            }
        }
        
        pub async fn user_action(
            &self,
            action: ServerUserAction
        ) -> Result<ServerUserResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client.user_action(context::current(), action.clone()).await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client.user_action(context::current(), action).await
                    .map_err(From::from)
            } else {
                result.map_err(From::from)
            }
        }
    }
    

}


