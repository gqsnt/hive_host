use serde::{Deserialize, Serialize};
use crate::server_action::permission::Permission;
use crate::server_action::project_action::{IsProjectServerAction};


#[derive(Debug,  Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum TokenAction {
    UploadFiles {path: String},
    UpdateFile{path: String},
    DownloadFile{path:String},
    DownloadDir{path:String},
}

impl IsProjectServerAction for TokenAction {
    fn permission(&self) -> Permission {
        match self {
            TokenAction::UpdateFile {..} | TokenAction::UploadFiles { .. } => Permission::Write,
            TokenAction::DownloadFile { .. } | TokenAction::DownloadDir { .. } => Permission::Read,
        }
    }

    fn require_csrf(&self) -> bool {
        match self {
            TokenAction::UpdateFile {..} |TokenAction::UploadFiles { .. }  => true,
            TokenAction::DownloadFile { .. } | TokenAction::DownloadDir { .. } => false,
        }
    }
}


pub type TokenActionResponse = String;


#[derive(Debug,  Clone, PartialEq, Eq,Deserialize,Serialize)]
pub enum UsedTokenActionResponse{
    Ok,
    File(FileInfo),
    UploadReport(Vec<FileUploadStatus>),
    Error(String),
}

#[derive( Debug, Clone, PartialEq, Eq,Deserialize,Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub content:Option<String>,
    pub size: u64,
    pub last_modified: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FileUploadStatus {
    pub filename: String,
    pub success: bool,
    pub message: String,
}