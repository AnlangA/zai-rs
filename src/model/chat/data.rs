use serde::{Deserialize, Serialize};

use super::ChatText;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Model>
where
    Model: ChatText,
{
    pub model: Model,
    pub content: String,
}
