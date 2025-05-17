use crate::{ProjectSlugStr, SanitizeError, SnapShotNameStr, UserSlugStr, Validate};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tarpc-server-to-helper")]
pub mod tarpc {
    use crate::helper_command::{HelperCommand, HelperResponse};
    use crate::tarpc_client::{TarpcClient, TarpcClientError};
    use crate::{AuthResponse, AuthToken, Validate};
    use tarpc::client::RpcError;
    use tarpc::context;

    pub const HELPER_SOCKET_PATH: &str = "/run/hivehost_server_helper/server_helper.sock";

    #[tarpc::service]
    pub trait ServerHelper {
        async fn execute(actions: Vec<HelperCommand>) -> HelperResponse;
        async fn auth(token: AuthToken) -> AuthResponse;
    }

    impl TarpcClient<ServerHelperClient> {
        pub async fn execute(
            &self,
            actions: Vec<HelperCommand>,
        ) -> Result<HelperResponse, TarpcClientError> {
            let client = self.get_or_connect_client().await?;
            let result = client.execute(context::current(), actions.clone()).await;
            if let Err(RpcError::Shutdown) = result {
                self.disconnect().await;
                let client = self.get_or_connect_client().await?;
                client
                    .execute(context::current(), actions)
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
pub enum HelperCommand {
    CreateUser {
        user_slug: UserSlugStr,
    },
    DeleteUser {
        user_slug: UserSlugStr,
    },
    CreateProject {
        project_slug: ProjectSlugStr,
        user_slug: UserSlugStr,
        with_index_html: bool,
    },
    DeleteProject {
        project_slug: ProjectSlugStr,
    },

    SetAcl {
        project_slug: ProjectSlugStr,
        user_slug: UserSlugStr,
        is_read_only: bool,
    },
    RemoveAcl {
        project_slug: ProjectSlugStr,
        user_slug: UserSlugStr,
    },

    BindMountUserProject {
        project_slug: ProjectSlugStr,
        user_slug: UserSlugStr,
    },
    UnmountUserProject {
        project_slug: ProjectSlugStr,
        user_slug: UserSlugStr,
    },

    CreateSnapshot {
        project_slug: ProjectSlugStr,
        snapshot_name: SnapShotNameStr,
    },
    DeleteSnapshot {
        snapshot_name: SnapShotNameStr,
    },
    RestoreSnapshot {
        project_slug: ProjectSlugStr,
        snapshot_name: SnapShotNameStr,
    },
    MountSnapshot {
        project_slug: ProjectSlugStr,
        snapshot_name: SnapShotNameStr,
    },
    UnmountProd {
        project_slug: ProjectSlugStr,
    },
}

impl Validate for HelperCommand {
    fn validate(&self) -> Result<(), SanitizeError> {
        match self {
            HelperCommand::CreateUser { user_slug } => {
                user_slug.validate()?;
            }
            HelperCommand::DeleteUser { user_slug } => {
                user_slug.validate()?;
            }
            HelperCommand::CreateProject {
                project_slug,
                user_slug,
                ..
            } => {
                project_slug.validate()?;
                user_slug.validate()?;
            }
            HelperCommand::DeleteProject { project_slug } => {
                project_slug.validate()?;
            }
            HelperCommand::SetAcl {
                project_slug,
                user_slug,
                ..
            } => {
                user_slug.validate()?;
                project_slug.validate()?;
            }
            HelperCommand::RemoveAcl {
                project_slug,
                user_slug,
            } => {
                user_slug.validate()?;
                project_slug.validate()?;
            }

            HelperCommand::BindMountUserProject {
                project_slug,
                user_slug,
            } => {
                user_slug.validate()?;
                project_slug.validate()?;
            }
            HelperCommand::UnmountUserProject {
                project_slug,
                user_slug,
            } => {
                user_slug.validate()?;
                project_slug.validate()?;
            }
            HelperCommand::CreateSnapshot {
                project_slug,
                snapshot_name,
            } => {
                project_slug.validate()?;
                snapshot_name.validate()?;
            }
            HelperCommand::DeleteSnapshot { snapshot_name } => {
                snapshot_name.validate()?;
            }
            HelperCommand::RestoreSnapshot {
                project_slug,
                snapshot_name,
            } => {
                project_slug.validate()?;
                snapshot_name.validate()?;
            }
            HelperCommand::MountSnapshot {
                project_slug,
                snapshot_name,
            } => {
                project_slug.validate()?;
                snapshot_name.validate()?;
            }
            HelperCommand::UnmountProd { project_slug } => {
                project_slug.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum HelperResponse {
    Ok,
    Error(String),
}
