use flutter_rust_bridge::frb;
use yrs::types::ToJson;
use yrs::{Array, ArrayPrelim, ArrayRef, Doc, In, MapPrelim, TextPrelim, Transact};

use crate::api::origin::local_txn;
use crate::api::values::{out_to_yout, YInValue, YOutValue};
use crate::api::yrs_map::YrsMap;
use crate::api::yrs_text::YrsText;

#[frb(opaque)]
#[derive(Clone)]
pub struct YrsArray {
    pub(crate) doc: Doc,
    pub(crate) inner: ArrayRef,
}

impl YrsArray {
    #[frb(sync)]
    pub fn insert(&self, index: u32, value: YInValue) {
        let mut txn = local_txn(&self.doc);
        self.inner.insert(&mut txn, index, In::from(value));
    }

    #[frb(sync)]
    pub fn insert_map(&self, index: u32) -> YrsMap {
        let mut txn = local_txn(&self.doc);
        let map_ref = self.inner.insert(&mut txn, index, MapPrelim::default());
        YrsMap {
            doc: self.doc.clone(),
            inner: map_ref,
        }
    }

    #[frb(sync)]
    pub fn insert_array(&self, index: u32) -> YrsArray {
        let mut txn = local_txn(&self.doc);
        let arr_ref = self.inner.insert(&mut txn, index, ArrayPrelim::default());
        YrsArray {
            doc: self.doc.clone(),
            inner: arr_ref,
        }
    }

    #[frb(sync)]
    pub fn insert_text(&self, index: u32) -> YrsText {
        let mut txn = local_txn(&self.doc);
        let text_ref = self.inner.insert(&mut txn, index, TextPrelim::new(""));
        YrsText {
            doc: self.doc.clone(),
            inner: text_ref,
        }
    }

    #[frb(sync)]
    pub fn push(&self, value: YInValue) {
        let mut txn = local_txn(&self.doc);
        self.inner.push_back(&mut txn, In::from(value));
    }

    #[frb(sync)]
    pub fn push_map(&self) -> YrsMap {
        let mut txn = local_txn(&self.doc);
        let map_ref = self.inner.push_back(&mut txn, MapPrelim::default());
        YrsMap {
            doc: self.doc.clone(),
            inner: map_ref,
        }
    }

    #[frb(sync)]
    pub fn push_array(&self) -> YrsArray {
        let mut txn = local_txn(&self.doc);
        let arr_ref = self.inner.push_back(&mut txn, ArrayPrelim::default());
        YrsArray {
            doc: self.doc.clone(),
            inner: arr_ref,
        }
    }

    #[frb(sync)]
    pub fn push_text(&self) -> YrsText {
        let mut txn = local_txn(&self.doc);
        let text_ref = self.inner.push_back(&mut txn, TextPrelim::new(""));
        YrsText {
            doc: self.doc.clone(),
            inner: text_ref,
        }
    }

    #[frb(sync)]
    pub fn remove_at(&self, index: u32) {
        let mut txn = local_txn(&self.doc);
        self.inner.remove(&mut txn, index);
    }

    #[frb(sync)]
    pub fn remove_range(&self, index: u32, length: u32) {
        let mut txn = local_txn(&self.doc);
        self.inner.remove_range(&mut txn, index, length);
    }

    #[frb(sync)]
    pub fn get(&self, index: u32) -> Option<YOutValue> {
        let txn = self.doc.transact();
        let out = self.inner.get(&txn, index)?;
        out_to_yout(out, &self.doc)
    }

    #[frb(sync)]
    pub fn length(&self) -> u32 {
        let txn = self.doc.transact();
        self.inner.len(&txn)
    }

    #[frb(sync)]
    pub fn json(&self) -> String {
        let txn = self.doc.transact();
        let any = self.inner.to_json(&txn);
        serde_json::to_string(&any).unwrap_or_else(|e| format!("<json error: {e}>"))
    }
}
