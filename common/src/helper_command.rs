use serde::{Deserialize, Serialize};
use crate::UserSlugStr;


#[cfg(feature = "tarpc-server-to-helper")]
pub mod tarpc{
    use tarpc::context;
    use tarpc::client::RpcError;
    use crate::helper_command::{HelperCommand, HelperResponse};
    use crate::tarpc_client::{TarpcClient, TarpcClientError};

    pub const HELPER_SOCKET_PATH: &str ="/run/hivehost_server_helper/server_helper.sock";

    #[tarpc::service]
    pub trait ServerHelper {
        async fn execute(actions: Vec<HelperCommand>) -> HelperResponse;
    }


    impl TarpcClient<ServerHelperClient>{
        pub async fn execute(
            &self,
            actions: Vec<HelperCommand>,
        ) -> Result<HelperResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client.execute(context::current(), actions.clone()).await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client.execute(context::current(), actions).await
                    .map_err(From::from)
            } else {
                result.map_err(From::from)
            }
        }
    }

}



#[derive(Debug, Clone, PartialEq, Eq, Deserialize,Serialize)]
pub enum HelperCommand {
    CreateUser {
        user_slug: UserSlugStr,
        user_path: String,
        user_projects_path: String,
    },
    DeleteUser {
        user_slug: UserSlugStr,
        user_path: String,
    },
    
    CreateProject {
        project_slug: String,
        service_user: String,
    },
    DeleteProject {
        project_slug: String,
    },

    SetAcl {
        path: String,
        user_slug: UserSlugStr,
        is_read_only: bool,
    },
    RemoveAcl {
        path: String,
        user_slug: UserSlugStr,
    },

    BindMountUserProject {
        source_path: String,
        target_path: String,
    },
    UnmountUserProject {
        target_path: String,
    },

    CreateSnapshot {
        path: String,
        snapshot_path: String,
    },
    DeleteSnapshot {
        snapshot_path: String,
    },
    RestoreSnapshot {
        path: String,
        snapshot_path: String,
        users_project_path:Vec<String>,
    },
    MountSnapshot {
        path: String,
        snapshot_name: String,
    },
    UnmountProd {
        path: String,
    },
}


#[derive(Debug, Clone, PartialEq, Eq, Deserialize,Serialize)]
pub enum HelperResponse {
    Ok,
    Error(String),
}

