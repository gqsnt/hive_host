use serde::{Deserialize, Serialize};

#[cfg(feature = "tarpc-server-to-hosting")]
pub mod tarpc {
    use crate::hosting_command::{HostingCommand, HostingResponse};
    use crate::tarpc_client::{TarpcClient, TarpcClientError};
    use crate::{AuthResponse, AuthToken, ProjectSlugStr, Validate};
    use tarpc::client::RpcError;
    use tarpc::context;
    pub const HOSTING_SOCKET_PATH: &str = "/run/hivehost_server_hosting/server_hosting.sock";
    #[tarpc::service]
    pub trait ServerHosting {
        async fn hosting(project_slug: ProjectSlugStr, action: HostingCommand) -> HostingResponse;

        async fn auth(token: AuthToken) -> AuthResponse;
    }

    impl TarpcClient<ServerHostingClient> {
        pub async fn hosting(
            &self,
            project_slug: ProjectSlugStr,
            action: HostingCommand,
        ) -> Result<HostingResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client
                .hosting(context::current(), project_slug.clone(), action.clone())
                .await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client
                    .hosting(context::current(), project_slug, action)
                    .await
                    .map_err(From::from)
            } else {
                result.map_err(From::from)
            }
        }

        pub async fn auth(&self, token: AuthToken) -> Result<bool, TarpcClientError> {
            if token.validate().is_err() {
                return Ok(false);
            }
            let client = self.get_or_connect_client().await?;
            match client.auth(context::current(), token.clone()).await {
                Ok(AuthResponse::Ok) => Ok(true),
                _ => Ok(false),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum HostingCommand {
    ServeReloadProject,
    StopServingProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum HostingResponse {
    Ok,
    Error(String),
}
