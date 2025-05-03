use std::fmt::Debug;



use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use crate::PING_PONG_ID;

pub trait ActionTrait:
Serialize + DeserializeOwned + Debug + Send + Sync + HasPing + 'static
{
}
impl<T: Serialize + DeserializeOwned + Debug + Send + Sync + HasPing + 'static> ActionTrait for T {}

pub trait ResponseTrait:
Serialize + DeserializeOwned + Debug + Send + Sync + HasPong + PartialEq + 'static
{
}
impl<T: Serialize + DeserializeOwned + Debug + Send + Sync + HasPong + PartialEq + 'static>
ResponseTrait for T
{
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MultiplexRequest<Action> {
    pub id: u64,
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MultiplexResponse<Response> {
    pub id: u64,
    pub action_response: Response,
}


pub type GenericRequest<Action> = MultiplexRequest<Action>;
pub type GenericResponse<Response> = MultiplexResponse<Response>;

pub trait HasPing:Sized {
    fn get_ping() -> Self;

    fn get_auth(_token:String) -> Option<Self> {
        None
    }
}

pub trait HasPong: PartialEq+ Sized + Debug {
    fn get_pong() -> Self;
    fn get_error(&self) -> Option<String>;

    fn make_error(msg: String) -> Self;
    
    
}

impl <Action:ActionTrait> GenericRequest<Action>{
    pub fn get_ping() -> Self {
        GenericRequest {
            id: PING_PONG_ID,
            action: Action::get_ping(),
        }
    }
    pub fn get_auth(token: String) -> Option<Self> {
        Action::get_auth(token).map(|action| GenericRequest {
            id: PING_PONG_ID,
            action,
        })
    }
    
}

impl <Response:ResponseTrait> GenericResponse<Response>{
    pub fn get_pong() -> Self {
        GenericResponse {
            id: PING_PONG_ID,
            action_response: Response::get_pong(),
        }
    }

    pub fn get_error(&self) -> Option<String> {
        self.action_response.get_error()
    }

    pub fn make_error(id:u64, msg: String) -> Self {
        GenericResponse {
            id,
            action_response: Response::make_error(msg),
        }
    }
}






#[cfg(feature = "website-to-server")]
impl HasPing for crate::website_to_server::WebSiteToServerAction {
    fn get_ping() -> Self {
        crate::website_to_server::WebSiteToServerAction::Ping
    }
    
    fn get_auth(_token: String) -> Option<Self> {
        Some(crate::website_to_server::WebSiteToServerAction::from_auth(_token))
    }
}

#[cfg(feature = "website-to-server")]
impl HasPong for crate::website_to_server::WebSiteToServerResponse {
    fn get_pong() -> Self {
        crate::website_to_server::WebSiteToServerResponse::Pong
    }

    fn get_error(&self) -> Option<String> {
        match self {
            crate::website_to_server::WebSiteToServerResponse::Error(err) => {
                Some(err.clone())
            }
            _ => None,
        }
    }
    fn make_error(msg: String) -> Self {
        crate::website_to_server::WebSiteToServerResponse::Error(msg)
    }
}

#[cfg(feature = "server-to-hosting")]
impl HasPing for crate::server::server_to_hosting::ServerToHostingAction {
    fn get_ping() -> Self {
        crate::server::server_to_hosting::ServerToHostingAction::Ping
    }
    
}

#[cfg(feature = "server-to-hosting")]
impl HasPong for crate::server::server_to_hosting::ServerToHostingResponse {
    fn get_pong() -> Self {
        crate::server::server_to_hosting::ServerToHostingResponse::Pong
    }

    fn get_error(&self) -> Option<String> {
        match self {
            crate::server::server_to_hosting::ServerToHostingResponse::Error(err) => Some(err.clone()),
            _ => None,
        }
    }

    fn make_error(msg: String) -> Self {
        crate::server::server_to_hosting::ServerToHostingResponse::Error(msg)
    }
    
}

#[cfg(feature = "server-to-helper")]
impl HasPing for crate::server::server_to_helper::ServerToHelperAction {
    fn get_ping() -> Self {
        crate::server::server_to_helper::ServerToHelperAction::Ping
    }
}

#[cfg(feature = "server-to-helper")]
impl HasPong for crate::server::server_to_helper::ServerToHelperResponse {
    fn get_pong() -> Self {
        crate::server::server_to_helper::ServerToHelperResponse::Pong
    }

    fn get_error(&self) -> Option<String> {
        match self {
            crate::server::server_to_helper::ServerToHelperResponse::Error(err) => Some(err.clone()),
            _ => None,
        }
    }

    fn make_error(msg: String) -> Self {
        crate::server::server_to_helper::ServerToHelperResponse::Error(msg)
    }
}