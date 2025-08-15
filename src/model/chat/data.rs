use super::super::base::*;
use super::super::traits::*;
use url::Url;
pub struct ChatCompletion<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    url: Url,
    body: ChatBody<N, M>,
}

impl<N, M> ChatCompletion<N, M>
where
    N: ModelName,
    (N, M): Bounded,
{
    pub fn new(model: N, messages: M) -> (Url, ChatBody<N, M>) {
        let url = Url::parse("https://open.bigmodel.cn/api/paas/v4/chat/completions").unwrap();
        let body = ChatBody::new(model, messages);
        (url, body)
    }
}
