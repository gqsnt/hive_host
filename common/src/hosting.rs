use bitcode::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum HostingAction {
    ServeReloadProject,
    StopServingProject,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum HostingResponse {
    Ok,
}
