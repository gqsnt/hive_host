use serde::{Deserialize, Serialize};
use crate::UserSlugStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHelperRequest {
    // Add a unique ID for tracing/correlation if needed
    // pub request_id: String,
    pub command: ServerHelperCommand,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerHelperCommand {
    // User Management
    CreateUser { user_slug:UserSlugStr, user_path:String, user_projects_path:String }, // Creates system user, home dir /sftp/users/user, adds to sftp_users
    DeleteUser { user_slug:UserSlugStr}, // Deletes system user, removes home dir

    // Project/ACL Management
    // Creates /projects/unix_slug owned by root:root, mode 700/750
    // Grants 'rwx' to SERVICE_USER via default and regular ACL
    CreateProjectDir { project_path: String, service_user: String },
    DeleteProjectDir { project_path: String },
    // Sets specific user ACLs on a project path
    SetAcl { path: String,user_slug:UserSlugStr, is_read_only:bool },
    RemoveAcl { path: String, user_slug:UserSlugStr },

    // Mount Management
    BindMount { source_path: String, target_path: String },
    Unmount { target_path: String },
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