use std::os::unix::net::Messages;

use super::super::base::*;
use super::ChatText;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Body<Model>
where
    Model: ChatText,
{
    model: Model,
    message: ChatMessages,
}
