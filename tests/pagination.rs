use zai_rs::{
    ZaiResult,
    batches::{BatchListQuery, BatchListRequest},
    file::{FileListPurpose, FileListQuery, FileListRequest},
    knowledge::{DocumentListRequest, KnowledgeListQuery, KnowledgeListRequest},
    pagination::{CursorPagination, PagePagination},
    services::assistants::{AssistantConversationListRequest, AssistantId},
};

#[test]
fn pagination_types_and_endpoint_methods_are_publicly_reachable() -> ZaiResult<()> {
    let cursor = CursorPagination::new()
        .try_with_after("opaque-cursor")?
        .try_with_limit(20)?;
    let _file = FileListRequest::new(FileListPurpose::Batch).try_with_pagination(cursor.clone())?;
    let _batch = BatchListRequest::new().try_with_pagination(cursor)?;

    let page = PagePagination::try_new(2, 50)?;
    let _knowledge = KnowledgeListRequest::new().try_with_pagination(page)?;
    let _documents = DocumentListRequest::new("knowledge-id").try_with_pagination(page)?;
    let _conversations =
        AssistantConversationListRequest::new(AssistantId::ChatGlm).try_with_pagination(page)?;

    Ok(())
}

#[test]
fn endpoint_specific_upper_bounds_remain_local() {
    let cursor = CursorPagination::new().try_with_limit(101).unwrap();
    assert!(
        FileListQuery::new(FileListPurpose::Batch)
            .try_with_pagination(cursor.clone())
            .is_err()
    );
    assert!(BatchListQuery::new().try_with_pagination(cursor).is_ok());

    let page = PagePagination::try_new(1, 101).unwrap();
    assert!(KnowledgeListQuery::new().try_with_pagination(page).is_ok());
    assert!(
        AssistantConversationListRequest::new(AssistantId::ChatGlm)
            .try_with_pagination(page)
            .is_err()
    );
}
