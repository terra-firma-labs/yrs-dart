use flutter_rust_bridge::frb;
use yrs::{Doc, GetString, Text, TextRef, Transact};

use crate::api::origin::local_txn;

#[frb(opaque)]
#[derive(Clone)]
pub struct YrsText {
    pub(crate) doc: Doc,
    pub(crate) inner: TextRef,
}

impl YrsText {
    #[frb(sync)]
    pub fn insert(&self, index: u32, chunk: String) {
        let mut txn = local_txn(&self.doc);
        self.inner.insert(&mut txn, index, &chunk);
    }

    #[frb(sync)]
    pub fn remove(&self, index: u32, length: u32) {
        let mut txn = local_txn(&self.doc);
        self.inner.remove_range(&mut txn, index, length);
    }

    /// Current string value. Indices are UTF-16 code units (matches Dart strings).
    #[frb(sync)]
    pub fn value(&self) -> String {
        let txn = self.doc.transact();
        self.inner.get_string(&txn)
    }

    /// Length in UTF-16 code units.
    #[frb(sync)]
    pub fn length(&self) -> u32 {
        let txn = self.doc.transact();
        self.inner.len(&txn)
    }
}
