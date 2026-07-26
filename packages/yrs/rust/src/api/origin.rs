//! Sentinel `Origin` tags for transactions originating from these bindings.
//!
//! The two-origin split lets `YrsUndoManager` track local user mutations while
//! ignoring remote-applied updates — preventing a user's Cmd-Z from undoing a
//! peer's edit in collab scenarios.

use yrs::{Doc, Transact, TransactionMut};

pub const YRS_DART_LOCAL_ORIGIN: &[u8] = b"yrs_dart/local";
pub const YRS_DART_REMOTE_ORIGIN: &[u8] = b"yrs_dart/remote";

/// Open a write transaction tagged with [`YRS_DART_LOCAL_ORIGIN`]. Used by
/// every direct Dart mutation so undo managers can track them.
pub(crate) fn local_txn(doc: &Doc) -> TransactionMut<'_> {
    doc.transact_mut_with(YRS_DART_LOCAL_ORIGIN)
}

/// Open a write transaction tagged with [`YRS_DART_REMOTE_ORIGIN`]. Used by
/// `YrsDoc::apply_update` so remote-applied updates stay out of the undo stack.
pub(crate) fn remote_txn(doc: &Doc) -> TransactionMut<'_> {
    doc.transact_mut_with(YRS_DART_REMOTE_ORIGIN)
}
