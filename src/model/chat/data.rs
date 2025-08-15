use super::super::base::*;
use super::super::traits::*;
pub struct ChatCompletion<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    body: ChatBody<N, M>,
}

impl<N, M> ChatCompletion<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    pub fn new(model: N, messages: M) -> (String, ChatBody<N, M>) {
        let url = "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string();
        let body = ChatBody::new(model, messages);
        (url, body)
    }
}
