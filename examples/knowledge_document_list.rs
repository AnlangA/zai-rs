use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Args: <knowledge_id> [word]
    let knowledge_id = std::env::args()
        .nth(1)
        .expect("Usage: knowledge_document_list <knowledge_id> [word]");
    let word_opt = std::env::args().nth(2);

    let mut q = DocumentListQuery::new(knowledge_id);
    if let Some(w) = word_opt {
        q = q.with_word(w);
    }

    let req = DocumentListRequest::new(key).with_query(q);
    let resp: DocumentListResponse = req.send().await?;

    println!(
        "code={:?} message={:?} timestamp={:?}",
        resp.code, resp.message, resp.timestamp
    );
    if let Some(data) = &resp.data {
        println!(
            "total={:?} list_len={}",
            data.total,
            data.list.as_ref().map(|v| v.len()).unwrap_or(0)
        );
        if let Some(list) = &data.list {
            for (i, d) in list.iter().enumerate().take(5) {
                println!(
                    "#{} id={:?} name={:?} words={:?} bytes={:?} stat={:?}",
                    i + 1,
                    d.id,
                    d.name,
                    d.word_num,
                    d.length,
                    d.embedding_stat
                );
            }
        }
    }

    Ok(())
}
