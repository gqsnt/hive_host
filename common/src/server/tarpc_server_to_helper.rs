use crate::server::server_to_helper::{ServerToHelperAction, ServerToHelperResponse};

#[tarpc::service]
pub trait ServerHelper {
    async fn execute(action: ServerToHelperAction) ->ServerToHelperResponse;
}
