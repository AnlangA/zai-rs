use zai_rs::knowledge::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY env var");

    // Optional: parse page/size from args
    let page = std::env::args().nth(1).and_then(|s| s.parse::<u32>().ok());
    let size = std::env::args().nth(2).and_then(|s| s.parse::<u32>().ok());

    let mut query = KnowledgeListQuery::new();
    if let Some(p) = page {
        query = query.with_page(p);
    }
    if let Some(s) = size {
        query = query.with_size(s);
    }

    let req = KnowledgeListRequest::new(key).with_query(query);
    let resp: KnowledgeListResponse = req.send().await?;

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
            for (i, item) in list.iter().enumerate().take(5) {
                println!(
                    "#{} id={:?} name={:?} docs={:?}",
                    i + 1,
                    item.id,
                    item.name,
                    item.document_size
                );
            }
        }
    }
    Ok(())
}
