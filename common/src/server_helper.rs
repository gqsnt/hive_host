use crate::UserSlugStr;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHelperRequest {
    // Add a unique ID for tracing/correlation if needed
    // pub request_id: String,
    pub command: ServerHelperCommand,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerHelperCommand {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHelperResponse {
    // pub corresponding_request_id: String, // Optional
    pub status: ServerHelperResponseStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerHelperResponseStatus {
    Success,
    Error(String), // Contains error message
}
