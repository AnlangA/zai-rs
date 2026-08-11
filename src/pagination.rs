//! Validated pagination values shared by list operations.
//!
//! These types model pagination semantics rather than one endpoint's wire
//! schema. Endpoints map
//! [`PagePagination::page_size`](crate::pagination::PagePagination::page_size)
//! to either `size` or `page_size`, so neither type implements Serde
//! serialization.
//!
//! ```
//! use zai_rs::{
//!     file::{FileListPurpose, FileListRequest},
//!     pagination::{CursorPagination, PagePagination},
//!     services::assistants::{AssistantConversationListRequest, AssistantId},
//! };
//!
//! # fn requests() -> zai_rs::ZaiResult<()> {
//! let files = FileListRequest::new(FileListPurpose::Batch).try_with_pagination(
//!     CursorPagination::new().try_with_limit(20)?,
//! )?;
//! let conversations = AssistantConversationListRequest::new(AssistantId::ChatGlm)
//!     .try_with_pagination(PagePagination::try_new(2, 50)?)?;
//! # let _ = (files, conversations);
//! # Ok(())
//! # }
//! ```

use std::num::NonZeroU32;

use crate::ZaiResult;

/// Cursor-based pagination with an optional starting cursor and page limit.
///
/// The first page normally omits `after`. Cursor values are preserved exactly
/// for transport but redacted from [`Debug`](std::fmt::Debug).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CursorPagination {
    after: Option<String>,
    limit: Option<NonZeroU32>,
}

impl CursorPagination {
    /// Create pagination for the first page using the endpoint's default limit.
    pub const fn new() -> Self {
        Self {
            after: None,
            limit: None,
        }
    }

    /// Set the cursor after validating that it is not blank.
    ///
    /// Leading and trailing whitespace is retained because cursors are opaque
    /// provider values; whitespace is inspected only to reject an empty value.
    pub fn try_with_after(mut self, after: impl Into<String>) -> ZaiResult<Self> {
        let after = after.into();
        if after.trim().is_empty() {
            return Err(crate::client::validation::invalid(
                "pagination cursor must not be blank",
            ));
        }
        self.after = Some(after);
        Ok(self)
    }

    /// Set a non-zero page limit.
    ///
    /// Endpoint-specific upper bounds are applied when this value is attached
    /// to a request query.
    pub fn try_with_limit(mut self, limit: u32) -> ZaiResult<Self> {
        self.limit = Some(NonZeroU32::new(limit).ok_or_else(|| {
            crate::client::validation::invalid("pagination limit must be at least 1")
        })?);
        Ok(self)
    }

    /// Borrow the opaque cursor, when one is configured.
    pub fn after(&self) -> Option<&str> {
        self.after.as_deref()
    }

    /// Return the configured page limit.
    pub const fn limit(&self) -> Option<u32> {
        match self.limit {
            Some(limit) => Some(limit.get()),
            None => None,
        }
    }

    pub(crate) fn into_parts(self) -> (Option<String>, Option<u32>) {
        (self.after, self.limit.map(NonZeroU32::get))
    }
}

impl Default for CursorPagination {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CursorPagination {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CursorPagination")
            .field("after", &self.after.as_ref().map(|_| "[REDACTED]"))
            .field("limit", &self.limit())
            .finish()
    }
}

/// One-based page pagination with an explicitly selected page size.
///
/// No global upper bound is imposed because endpoint limits differ. Assistant
/// conversation listing applies its own maximum when pagination is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PagePagination {
    page: NonZeroU32,
    page_size: NonZeroU32,
}

impl PagePagination {
    /// Create pagination from a one-based page and non-zero page size.
    pub fn try_new(page: u32, page_size: u32) -> ZaiResult<Self> {
        let page = NonZeroU32::new(page).ok_or_else(|| {
            crate::client::validation::invalid("pagination page must be at least 1")
        })?;
        let page_size = NonZeroU32::new(page_size).ok_or_else(|| {
            crate::client::validation::invalid("pagination page size must be at least 1")
        })?;
        Ok(Self { page, page_size })
    }

    /// Return the one-based page number.
    pub const fn page(&self) -> u32 {
        self.page.get()
    }

    /// Return the page size.
    pub const fn page_size(&self) -> u32 {
        self.page_size.get()
    }

    pub(crate) const fn into_parts(self) -> (u32, u32) {
        (self.page.get(), self.page_size.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ZaiError, client::error::codes};

    fn assert_validation(error: ZaiError) {
        assert!(matches!(
            error,
            ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                ..
            }
        ));
    }

    #[test]
    fn cursor_pagination_validates_and_preserves_opaque_values() {
        for blank in ["", " \t\n", "\u{2003}\u{3000}"] {
            assert_validation(CursorPagination::new().try_with_after(blank).unwrap_err());
        }

        let cursor = "  游标 &/?=+%  ";
        let pagination = CursorPagination::new()
            .try_with_after(cursor)
            .unwrap()
            .try_with_limit(1)
            .unwrap();
        assert_eq!(pagination.after(), Some(cursor));
        assert_eq!(pagination.limit(), Some(1));

        let debug = format!("{pagination:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(cursor));
    }

    #[test]
    fn cursor_limit_checks_zero_and_accepts_the_u32_domain() {
        assert_validation(CursorPagination::new().try_with_limit(0).unwrap_err());
        assert_eq!(
            CursorPagination::new()
                .try_with_limit(u32::MAX)
                .unwrap()
                .limit(),
            Some(u32::MAX)
        );
    }

    #[test]
    fn page_pagination_checks_both_non_zero_fields() {
        assert_validation(PagePagination::try_new(0, 1).unwrap_err());
        assert_validation(PagePagination::try_new(1, 0).unwrap_err());

        let pagination = PagePagination::try_new(1, u32::MAX).unwrap();
        assert_eq!(pagination.page(), 1);
        assert_eq!(pagination.page_size(), u32::MAX);
        let copied = pagination;
        assert_eq!(copied, pagination);
    }
}
