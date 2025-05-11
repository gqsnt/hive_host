use serde::{Deserialize, Serialize};


#[cfg(feature = "tarpc-server-to-hosting")]
pub mod tarpc{
    use tarpc::context;
    use tarpc::client::RpcError;
    use crate::hosting_command::{HostingCommand, HostingResponse};
    use crate::ProjectSlugStr;
    use crate::tarpc_client::{TarpcClient, TarpcClientError};
    pub const HOSTING_SOCKET_PATH: &str ="/run/hivehost_server_hosting/server_hosting.sock";
    #[tarpc::service]
    pub trait ServerHosting {
        async fn hosting(project_slug:ProjectSlugStr, action: HostingCommand) -> HostingResponse;
    }


    impl TarpcClient<ServerHostingClient>{
        pub async fn hosting(
            &self,
            project_slug:ProjectSlugStr,
            action: HostingCommand
        ) -> Result<HostingResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client.hosting(context::current(),project_slug.clone(), action.clone()).await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client.hosting(context::current(),project_slug, action).await
                    .map_err(From::from)
            } else {
                result.map_err(From::from)
            }
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum HostingCommand {
    ServeReloadProject,
    StopServingProject,
}

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum HostingResponse {
    Ok,
    Error(String),
}
