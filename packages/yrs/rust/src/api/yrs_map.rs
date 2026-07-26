use flutter_rust_bridge::frb;
use yrs::types::ToJson;
use yrs::{Doc, In, Map, MapPrelim, MapRef, Transact};

use crate::api::origin::local_txn;
use crate::api::values::{out_to_yout, YInValue, YOutValue};
use crate::api::yrs_array::YrsArray;
use crate::api::yrs_text::YrsText;

#[frb(opaque)]
#[derive(Clone)]
pub struct YrsMap {
    pub(crate) doc: Doc,
    pub(crate) inner: MapRef,
}

impl YrsMap {
    #[frb(sync)]
    pub fn set(&self, key: String, value: YInValue) {
        let mut txn = local_txn(&self.doc);
        self.inner.insert(&mut txn, key, In::from(value));
    }

    #[frb(sync)]
    pub fn set_map(&self, key: String) -> YrsMap {
        let mut txn = local_txn(&self.doc);
        let map_ref = self.inner.insert(&mut txn, key, MapPrelim::default());
        YrsMap {
            doc: self.doc.clone(),
            inner: map_ref,
        }
    }

    #[frb(sync)]
    pub fn set_array(&self, key: String) -> YrsArray {
        let mut txn = local_txn(&self.doc);
        let arr_ref = self
            .inner
            .insert(&mut txn, key, yrs::ArrayPrelim::default());
        YrsArray {
            doc: self.doc.clone(),
            inner: arr_ref,
        }
    }

    #[frb(sync)]
    pub fn set_text(&self, key: String) -> YrsText {
        let mut txn = local_txn(&self.doc);
        let text_ref = self.inner.insert(&mut txn, key, yrs::TextPrelim::new(""));
        YrsText {
            doc: self.doc.clone(),
            inner: text_ref,
        }
    }

    #[frb(sync)]
    pub fn delete(&self, key: String) {
        let mut txn = local_txn(&self.doc);
        self.inner.remove(&mut txn, &key);
    }

    #[frb(sync)]
    pub fn clear(&self) {
        let mut txn = local_txn(&self.doc);
        self.inner.clear(&mut txn);
    }

    #[frb(sync)]
    pub fn get(&self, key: String) -> Option<YOutValue> {
        let txn = self.doc.transact();
        let out = self.inner.get(&txn, &key)?;
        out_to_yout(out, &self.doc)
    }

    #[frb(sync)]
    pub fn contains(&self, key: String) -> bool {
        let txn = self.doc.transact();
        self.inner.contains_key(&txn, &key)
    }

    #[frb(sync)]
    pub fn length(&self) -> u32 {
        let txn = self.doc.transact();
        self.inner.len(&txn)
    }

    #[frb(sync)]
    pub fn keys(&self) -> Vec<String> {
        let txn = self.doc.transact();
        self.inner.keys(&txn).map(|k| k.to_string()).collect()
    }

    #[frb(sync)]
    pub fn json(&self) -> String {
        let txn = self.doc.transact();
        let any = self.inner.to_json(&txn);
        serde_json::to_string(&any).unwrap_or_else(|e| format!("<json error: {e}>"))
    }
}
