use serde::{Deserialize, Serialize};
use crate::UserSlugStr;




#[derive(Debug, Clone, PartialEq, Eq, Deserialize,Serialize)]
pub enum ServerToHelperAction {
    CreateUser {
        user_slug: UserSlugStr,
        user_path: String,
        user_projects_path: String,
    }, // Creates system user, home dir /sftp/users/user, adds to sftp_users
    DeleteUser {
        user_slug: UserSlugStr,
    }, // Deletes system user, removes home dir

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
    MountSnapshot {
        path: String,
        snapshot_name: String,
    },
    UnmountProd {
        path: String,
    },
}


#[derive(Debug, Clone, PartialEq, Eq, Deserialize,Serialize)]
pub enum ServerToHelperResponse {
    Ok,
    Error(String),
}


