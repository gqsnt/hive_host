use crate::hosting::{HostingAction, HostingResponse};

#[tarpc::service]
pub trait ServerHosting {
    async fn execute(project_slug:String, action: HostingAction) -> HostingResponse;
}