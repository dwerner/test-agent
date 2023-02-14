// pub use casper_client;
// pub use casper_node;
// pub use casper_types;

use tarpc::serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    HelloWorld(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Success,
    Error,
}

#[tarpc::service]
pub trait AgentService {
    async fn message(msg: Message) -> Response;
}
