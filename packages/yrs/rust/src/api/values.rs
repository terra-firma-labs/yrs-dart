use yrs::{Any, Doc, In, Out};

use crate::api::yrs_array::YrsArray;
use crate::api::yrs_map::YrsMap;
use crate::api::yrs_text::YrsText;

pub enum YInValue {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
    Null,
    Bytes(Vec<u8>),
}

impl From<YInValue> for In {
    fn from(v: YInValue) -> Self {
        match v {
            YInValue::String(s) => In::from(s),
            // Dart's `int` contract requires a distinct persisted scalar kind.
            // `In::from(i64)` collapses JS-safe values to `Any::Number`, so use
            // Yjs/lib0's standard signed BigInt representation explicitly.
            // JavaScript peers therefore observe these authored values as
            // `bigint` (and must not pass them directly to `JSON.stringify`).
            YInValue::Int(i) => In::Any(Any::BigInt(i)),
            YInValue::Double(f) => In::from(f),
            YInValue::Bool(b) => In::from(b),
            YInValue::Bytes(b) => In::from(b),
            YInValue::Null => In::Any(Any::Null),
        }
    }
}

pub enum YOutValue {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
    Null,
    Bytes(Vec<u8>),
    /// Frozen `Any::Array` serialized as JSON; Dart side decodes to `List<Object?>`.
    JsonArray(String),
    /// Frozen `Any::Map` serialized as JSON; Dart side decodes to `Map<String, Object?>`.
    JsonMap(String),
    Map(YrsMap),
    Array(YrsArray),
    Text(YrsText),
}

/// Variant tag for `YOutValue`, returned by its `kind` accessor so the Dart
/// wrapper can dispatch on the correct accessor in one FFI call instead of
/// probing each `as_*` accessor in sequence.
pub enum YOutKind {
    String,
    Int,
    Double,
    Bool,
    Null,
    Bytes,
    JsonArray,
    JsonMap,
    Map,
    Array,
    Text,
}

impl YOutValue {
    #[flutter_rust_bridge::frb(sync)]
    pub fn kind(&self) -> YOutKind {
        match self {
            YOutValue::String(_) => YOutKind::String,
            YOutValue::Int(_) => YOutKind::Int,
            YOutValue::Double(_) => YOutKind::Double,
            YOutValue::Bool(_) => YOutKind::Bool,
            YOutValue::Null => YOutKind::Null,
            YOutValue::Bytes(_) => YOutKind::Bytes,
            YOutValue::JsonArray(_) => YOutKind::JsonArray,
            YOutValue::JsonMap(_) => YOutKind::JsonMap,
            YOutValue::Map(_) => YOutKind::Map,
            YOutValue::Array(_) => YOutKind::Array,
            YOutValue::Text(_) => YOutKind::Text,
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_string(&self) -> Option<String> {
        if let YOutValue::String(s) = self {
            Some(s.clone())
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_int(&self) -> Option<i64> {
        if let YOutValue::Int(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_double(&self) -> Option<f64> {
        if let YOutValue::Double(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_bool(&self) -> Option<bool> {
        if let YOutValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn is_null(&self) -> bool {
        matches!(self, YOutValue::Null)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        if let YOutValue::Bytes(b) = self {
            Some(b.clone())
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_json_array(&self) -> Option<String> {
        if let YOutValue::JsonArray(s) = self {
            Some(s.clone())
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_json_map(&self) -> Option<String> {
        if let YOutValue::JsonMap(s) = self {
            Some(s.clone())
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_map(&self) -> Option<YrsMap> {
        if let YOutValue::Map(m) = self {
            Some(m.clone())
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_array(&self) -> Option<YrsArray> {
        if let YOutValue::Array(a) = self {
            Some(a.clone())
        } else {
            None
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn as_text(&self) -> Option<YrsText> {
        if let YOutValue::Text(t) = self {
            Some(t.clone())
        } else {
            None
        }
    }
}

pub(crate) fn out_to_yout(out: Out, doc: &Doc) -> Option<YOutValue> {
    Some(match out {
        Out::Any(any) => any_to_yout(&any),
        Out::YMap(map_ref) => YOutValue::Map(YrsMap {
            doc: doc.clone(),
            inner: map_ref,
        }),
        Out::YArray(arr_ref) => YOutValue::Array(YrsArray {
            doc: doc.clone(),
            inner: arr_ref,
        }),
        Out::YText(text_ref) => YOutValue::Text(YrsText {
            doc: doc.clone(),
            inner: text_ref,
        }),
        // Unsupported Out variants (XML, sub-doc, weak/undef) surface as None; no typed error path yet.
        _ => return None,
    })
}

fn any_to_yout(any: &Any) -> YOutValue {
    match any {
        Any::Null | Any::Undefined => YOutValue::Null,
        Any::Bool(b) => YOutValue::Bool(*b),
        Any::Number(n) => YOutValue::Double(*n),
        Any::BigInt(i) => YOutValue::Int(*i),
        Any::String(s) => YOutValue::String(s.to_string()),
        Any::Buffer(b) => YOutValue::Bytes(b.to_vec()),
        Any::Array(_) => YOutValue::JsonArray(
            serde_json::to_string(any)
                .expect("Any::Array/Map serialization is infallible for these variants"),
        ),
        Any::Map(_) => YOutValue::JsonMap(
            serde_json::to_string(any)
                .expect("Any::Array/Map serialization is infallible for these variants"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_ints_use_bigint_without_reclassifying_numbers() {
        for value in [
            i64::MIN,
            -9_007_199_254_740_992,
            -9_007_199_254_740_991,
            -12,
            0,
            12,
            9_007_199_254_740_991,
            9_007_199_254_740_992,
            i64::MAX,
        ] {
            assert_eq!(In::from(YInValue::Int(value)), In::Any(Any::BigInt(value)));
        }

        assert_eq!(In::from(YInValue::Double(12.0)), In::Any(Any::Number(12.0)));
    }

    #[test]
    fn external_number_and_bigint_variants_keep_their_kinds() {
        assert!(matches!(
            any_to_yout(&Any::Number(12.0)),
            YOutValue::Double(value) if value == 12.0
        ));
        assert!(matches!(any_to_yout(&Any::BigInt(12)), YOutValue::Int(12)));
    }
}
