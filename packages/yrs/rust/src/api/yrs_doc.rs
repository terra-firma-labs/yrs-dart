use std::sync::Mutex;

use flutter_rust_bridge::frb;

use crate::frb_generated::StreamSink;
use yrs::types::text::{Text, YChange};
use yrs::types::xml::{Xml, XmlFragment, XmlOut};
use yrs::types::AsPrelim;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{
    Array, ArrayPrelim, ArrayRef, Doc, In, Map, MapRef, OffsetKind, Options, Out, ReadTxn,
    StateVector, Subscription, Transact, Update,
};

use crate::api::origin::{local_txn, remote_txn};
use crate::api::yrs_array::YrsArray;
use crate::api::yrs_map::YrsMap;
use crate::api::yrs_text::YrsText;

/// A Yjs/yrs document.
///
/// Wraps a [`yrs::Doc`] plus a vec of held [`yrs::Subscription`]s that keep
/// observer callbacks alive for the lifetime of this handle. `Doc` uses
/// interior mutability internally (it is `Clone + Send + Sync` and `Arc`-backed)
/// so most methods take `&self`.
///
/// Configured with [`OffsetKind::Utf16`] so YText indices match Dart string
/// indices (Dart strings are UTF-16 internally).
#[frb(opaque)]
pub struct YrsDoc {
    pub(crate) inner: Doc,
    /// Held to keep yrs observer callbacks alive. Interior `Mutex` because
    /// `observe_*` methods take `&self`.
    subscriptions: Mutex<Vec<Subscription>>,
}

/// What an atomic document transfer actually did.
///
/// A transfer has two possible mechanisms, and which one runs is not something
/// the caller chooses — it follows from whether the entry stays under the same
/// parent. The distinction matters for collaboration, so it is reported rather
/// than hidden.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YrsTransferOutcome {
    /// Nothing was written: the request was already satisfied. A same-list move
    /// to either adjacent insertion gap, or a move to the map slot the entry
    /// already occupies.
    Unchanged,

    /// A real CRDT move within one list. The entry keeps its identity: a peer's
    /// concurrent edits to it survive synchronization, live container handles to
    /// it stay valid, and concurrent peers converge on the move itself.
    Moved,

    /// The entry was deep-copied under its new parent and the original removed,
    /// because neither yrs nor Yjs can express a move between different parents
    /// — there is no cross-parent move in the CRDT to fall back on.
    ///
    /// Two consequences follow, and both are visible to users:
    ///
    /// - A peer's edits to the moved subtree made concurrently with the reparent
    ///   are **discarded** once the two sides synchronize. The subtree that
    ///   arrives is a copy of the state this peer saw.
    /// - Container handles held on the source become **invalid**. Reads return
    ///   nothing and writes land on a deleted branch, growing the document
    ///   without producing any visible change.
    ///
    /// A caller that cares should re-resolve handles after a `Reparented`
    /// outcome, and should think about whether reparenting is safe to offer at
    /// all while remote peers may be editing the same subtree.
    Reparented,

    /// The entry was deep-copied and the source left in place. The copy is
    /// independent: mutating either side does not affect the other.
    Copied,
}

impl YrsTransferOutcome {
    /// Whether the document was modified. `false` only for the `unchanged`
    /// outcome.
    #[frb(sync)]
    pub fn changed_document(&self) -> bool {
        !matches!(self, YrsTransferOutcome::Unchanged)
    }
}

/// Whether an atomic document transfer removes or preserves its source entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YrsTransferMode {
    Move,
    Copy,
}

/// One typed step through a nested yrs map/array document tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YrsPathSegment {
    Key(String),
    Index(u32),
}

/// A source or destination entry in an array or map.
///
/// Array indexes are insertion gaps when used as a destination. Map keys are
/// single-child slots and must be empty before a copy or non-trivial move.
pub enum YrsTransferLocation {
    Array {
        path: Vec<YrsPathSegment>,
        index: u32,
    },
    Map {
        path: Vec<YrsPathSegment>,
        key: String,
    },
    /// Destination-only list slot owned by a map. If [key] is absent, an
    /// array is created inside the same validated transfer transaction. An
    /// existing non-array value is always rejected before the first write.
    ArrayAtMapKey {
        parent_path: Vec<YrsPathSegment>,
        key: String,
        index: u32,
    },
}

impl YrsDoc {
    fn new_with_options() -> Doc {
        Doc::with_options(Options {
            offset_kind: OffsetKind::Utf16,
            ..Default::default()
        })
    }

    /// Create a new empty document.
    #[frb(sync)]
    pub fn new_empty() -> YrsDoc {
        YrsDoc {
            inner: Self::new_with_options(),
            subscriptions: Mutex::new(Vec::new()),
        }
    }

    /// Reload a document from a previously-saved binary blob.
    #[frb(sync)]
    pub fn from_bytes(blob: Vec<u8>) -> Result<YrsDoc, String> {
        let inner = Self::new_with_options();
        let update = Update::decode_v1(&blob).map_err(|e| e.to_string())?;
        // Hydration is tagged remote so any later-attached UndoManager would
        // not include the rehydrated initial state on its undo stack.
        remote_txn(&inner)
            .apply_update(update)
            .map_err(|e| e.to_string())?;
        Ok(YrsDoc {
            inner,
            subscriptions: Mutex::new(Vec::new()),
        })
    }

    /// Save the document to a binary blob. Symmetric with `from_bytes`.
    #[frb(sync)]
    pub fn save(&self) -> Vec<u8> {
        self.inner
            .transact()
            .encode_state_as_update_v1(&Default::default())
    }

    /// Get-or-create a root-level map by name.
    #[frb(sync)]
    pub fn get_map(&self, name: String) -> YrsMap {
        let map_ref = self.inner.get_or_insert_map(name);
        YrsMap {
            doc: self.inner.clone(),
            inner: map_ref,
        }
    }

    /// Get-or-create a root-level array by name.
    #[frb(sync)]
    pub fn get_array(&self, name: String) -> YrsArray {
        let arr_ref = self.inner.get_or_insert_array(name);
        YrsArray {
            doc: self.inner.clone(),
            inner: arr_ref,
        }
    }

    /// Get-or-create a root-level text by name.
    #[frb(sync)]
    pub fn get_text(&self, name: String) -> YrsText {
        let text_ref = self.inner.get_or_insert_text(name);
        YrsText {
            doc: self.inner.clone(),
            inner: text_ref,
        }
    }

    /// Apply a v1-encoded update from a remote peer or storage rehydration.
    /// Tagged with the remote origin so it never enters any active
    /// `YUndoManager`'s undo stack.
    #[frb(sync)]
    pub fn apply_update(&self, update: Vec<u8>) -> Result<(), String> {
        let parsed = Update::decode_v1(&update).map_err(|e| e.to_string())?;
        remote_txn(&self.inner)
            .apply_update(parsed)
            .map_err(|e| e.to_string())
    }

    /// Snapshot of the local clock state, encoded as v1 bytes. Send to a peer
    /// who can compute the diff this doc is missing via `encodeStateAsUpdate`.
    #[frb(sync)]
    pub fn get_state_vector(&self) -> Vec<u8> {
        self.inner.transact().state_vector().encode_v1()
    }

    /// Encode operations this doc has that the given `state_vector` does not.
    /// If `state_vector` is `None`, encodes the full state (equivalent to
    /// `save()`).
    #[frb(sync)]
    pub fn encode_state_as_update(&self, state_vector: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
        let sv = match state_vector {
            Some(bytes) => StateVector::decode_v1(&bytes).map_err(|e| e.to_string())?,
            None => StateVector::default(),
        };
        Ok(self.inner.transact().encode_state_as_update_v1(&sv))
    }

    /// Move or deep-copy one document entry using one local yrs transaction.
    ///
    /// All paths, container kinds, source existence, target availability,
    /// indexes, and ancestry are validated against that same transaction before
    /// its first write, so a rejected transfer leaves the document byte-identical.
    /// A same-list move to either adjacent insertion gap is a zero-write no-op.
    ///
    /// **A move within one list and a move between lists are not the same
    /// operation.** The first is a real CRDT move; the second is a deep copy plus
    /// a removal, because no cross-parent move exists in yrs or Yjs. The second
    /// therefore discards a peer's concurrent edits to the moved subtree and
    /// invalidates live handles to it. The returned [`YrsTransferOutcome`] says
    /// which happened — inspect it rather than assuming, particularly if remote
    /// peers may be editing the same subtree.
    #[frb(sync)]
    pub fn transfer_entry_atomically(
        &self,
        mode: YrsTransferMode,
        source: YrsTransferLocation,
        target: YrsTransferLocation,
    ) -> Result<YrsTransferOutcome, String> {
        let mut txn = local_txn(&self.inner);
        let source = ResolvedLocation::resolve(&txn, source, "source")?;
        let target = ResolvedLocation::resolve(&txn, target, "target")?;

        if source.target_descends_into_source(&target) {
            return Err("target path descends into the source entry".to_owned());
        }
        source.validate_source(&txn)?;

        if mode == YrsTransferMode::Move && source.is_same_map_entry(&target) {
            return Ok(YrsTransferOutcome::Unchanged);
        }
        target.validate_target(&txn)?;

        if mode == YrsTransferMode::Move {
            if let (Some((source_array, source_index)), Some((target_array, target_index))) =
                (source.array_entry(), target.array_entry())
            {
                if source_array == target_array {
                    if source_index == target_index
                        || source_index.checked_add(1) == Some(target_index)
                    {
                        return Ok(YrsTransferOutcome::Unchanged);
                    }
                    source_array.move_to(&mut txn, source_index, target_index);
                    return Ok(YrsTransferOutcome::Moved);
                }
                // Different parents: fall through to the reparent path below.
                // Neither yrs nor Yjs can express a move across parents, so
                // there is no CRDT move to reach for here.
            }
        }

        let transferred = source.deep_copy(&txn)?;
        let outcome = if mode == YrsTransferMode::Move {
            source.remove(&mut txn);
            YrsTransferOutcome::Reparented
        } else {
            YrsTransferOutcome::Copied
        };
        target.insert(&mut txn, transferred);
        Ok(outcome)
    }

    /// Subscribe to v1-encoded update blobs. The `sink` receives one event per
    /// committed transaction. Each blob is consumable by `apply_update` on a
    /// peer doc. The subscription is held for the lifetime of this `YrsDoc`
    /// (or until `dispose()`).
    ///
    /// `#[frb(sync)]` is required: `default_dart_async: false` skips the frb
    /// worker pool, so an async-dispatched call would have no executor.
    #[frb(sync)]
    pub fn observe_updates(&self, sink: StreamSink<Vec<u8>>) -> Result<(), String> {
        let sub = self
            .inner
            .observe_update_v1(move |_txn, evt| {
                // Err means the Dart sink was cancelled; yrs keeps firing
                // for the doc's lifetime regardless.
                let _ = sink.add(evt.update.clone());
            })
            .map_err(|e| e.to_string())?;
        self.subscriptions.lock().unwrap().push(sub);
        Ok(())
    }

    /// Release held observer subscriptions. yrs's `Doc` itself is `Arc`-backed
    /// and freed when the last handle drops; this method exists for symmetry
    /// with consumer lifecycle code (e.g. `Bloc.close`).
    #[frb(sync)]
    pub fn dispose(&self) {
        self.subscriptions.lock().unwrap().clear();
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ContainerKind {
    Map,
    Array,
}

enum PathContainer {
    Map(MapRef),
    Array(ArrayRef),
}

enum ResolvedLocation {
    Array {
        path: Vec<YrsPathSegment>,
        array: ArrayRef,
        index: u32,
    },
    Map {
        path: Vec<YrsPathSegment>,
        map: MapRef,
        key: String,
    },
    ArrayAtMapKey {
        path: Vec<YrsPathSegment>,
        map: MapRef,
        key: String,
        array: Option<ArrayRef>,
        index: u32,
    },
}

impl ResolvedLocation {
    fn array_entry(&self) -> Option<(&ArrayRef, u32)> {
        match self {
            Self::Array { array, index, .. } => Some((array, *index)),
            Self::ArrayAtMapKey {
                array: Some(array),
                index,
                ..
            } => Some((array, *index)),
            _ => None,
        }
    }

    fn resolve<T: ReadTxn>(
        txn: &T,
        location: YrsTransferLocation,
        label: &str,
    ) -> Result<Self, String> {
        match location {
            YrsTransferLocation::Array { path, index } => {
                let container = resolve_container(txn, &path, ContainerKind::Array, label)?;
                let PathContainer::Array(array) = container else {
                    unreachable!("resolve_container enforces the requested kind");
                };
                Ok(Self::Array { path, array, index })
            }
            YrsTransferLocation::Map { path, key } => {
                let container = resolve_container(txn, &path, ContainerKind::Map, label)?;
                let PathContainer::Map(map) = container else {
                    unreachable!("resolve_container enforces the requested kind");
                };
                Ok(Self::Map { path, map, key })
            }
            YrsTransferLocation::ArrayAtMapKey {
                parent_path,
                key,
                index,
            } => {
                let container = resolve_container(txn, &parent_path, ContainerKind::Map, label)?;
                let PathContainer::Map(map) = container else {
                    unreachable!("resolve_container enforces the requested kind");
                };
                let array = match map.get(txn, key.as_str()) {
                    None => None,
                    Some(Out::YArray(array)) => Some(array),
                    Some(other) => {
                        return Err(format!(
                            "{label} map key {key:?} resolves to {}, not an array",
                            root_kind_label(&other)
                        ));
                    }
                };
                let mut path = parent_path;
                path.push(YrsPathSegment::Key(key.clone()));
                Ok(Self::ArrayAtMapKey {
                    path,
                    map,
                    key,
                    array,
                    index,
                })
            }
        }
    }

    fn validate_source<T: ReadTxn>(&self, txn: &T) -> Result<(), String> {
        match self {
            Self::Array { array, index, .. } => {
                let len = array.len(txn);
                if *index < len {
                    Ok(())
                } else {
                    Err(format!(
                        "source array index {index} is outside entry range 0..{len}"
                    ))
                }
            }
            Self::Map { map, key, .. } => {
                if map.get(txn, key).is_some() {
                    Ok(())
                } else {
                    Err(format!("source map key {key:?} does not exist"))
                }
            }
            Self::ArrayAtMapKey { .. } => {
                Err("array-at-map-key is a destination-only location".to_owned())
            }
        }
    }

    fn validate_target<T: ReadTxn>(&self, txn: &T) -> Result<(), String> {
        match self {
            Self::Array { array, index, .. } => {
                let len = array.len(txn);
                if *index <= len {
                    Ok(())
                } else {
                    Err(format!(
                        "target array index {index} is outside insertion range 0..={len}"
                    ))
                }
            }
            Self::Map { map, key, .. } => {
                if map.get(txn, key).is_none() {
                    Ok(())
                } else {
                    Err(format!("target map key {key:?} is already occupied"))
                }
            }
            Self::ArrayAtMapKey { array, index, .. } => {
                let len = array.as_ref().map(|value| value.len(txn)).unwrap_or(0);
                if *index <= len {
                    Ok(())
                } else {
                    Err(format!(
                        "target array index {index} is outside insertion range 0..={len}"
                    ))
                }
            }
        }
    }

    fn target_descends_into_source(&self, target: &Self) -> bool {
        let source_path = self.path();
        if !target.path().starts_with(source_path) {
            return false;
        }
        match (self, target.path().get(source_path.len())) {
            (Self::Array { index, .. }, Some(YrsPathSegment::Index(next))) => index == next,
            (Self::Map { key, .. }, Some(YrsPathSegment::Key(next))) => key == next,
            _ => false,
        }
    }

    fn path(&self) -> &[YrsPathSegment] {
        match self {
            Self::Array { path, .. }
            | Self::Map { path, .. }
            | Self::ArrayAtMapKey { path, .. } => path,
        }
    }

    fn is_same_map_entry(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (
                Self::Map {
                    map: source_map,
                    key: source_key,
                    ..
                },
                Self::Map {
                    map: target_map,
                    key: target_key,
                    ..
                }
            ) if source_map == target_map && source_key == target_key
        )
    }

    fn deep_copy<T: ReadTxn>(&self, txn: &T) -> Result<In, String> {
        match self {
            Self::Array { array, index, .. } => array.get(txn, *index),
            Self::Map { map, key, .. } => map.get(txn, key),
            Self::ArrayAtMapKey { .. } => None,
        }
        .ok_or_else(|| "source entry disappeared before mutation".to_owned())
        .and_then(|out| {
            // Measured iteratively, before the copy and before any write, so a
            // pathological source is rejected like any other invalid transfer
            // instead of aborting the process.
            validate_copy_depth(txn, &out, MAX_TRANSFER_DEPTH)?;
            Ok(out.as_prelim(txn))
        })
    }

    fn remove(&self, txn: &mut yrs::TransactionMut<'_>) {
        match self {
            Self::Array { array, index, .. } => array.remove(txn, *index),
            Self::Map { map, key, .. } => {
                map.remove(txn, key);
            }
            Self::ArrayAtMapKey { .. } => {
                unreachable!("array-at-map-key cannot be a source location")
            }
        }
    }

    fn insert(&self, txn: &mut yrs::TransactionMut<'_>, value: In) {
        match self {
            Self::Array { array, index, .. } => {
                array.insert(txn, *index, value);
            }
            Self::Map { map, key, .. } => {
                map.insert(txn, key.as_str(), value);
            }
            Self::ArrayAtMapKey {
                map,
                key,
                array,
                index,
                ..
            } => {
                let array = array
                    .clone()
                    .unwrap_or_else(|| map.insert(txn, key.as_str(), ArrayPrelim::default()));
                array.insert(txn, *index, value);
            }
        }
    }
}

fn resolve_container<T: ReadTxn>(
    txn: &T,
    path: &[YrsPathSegment],
    expected: ContainerKind,
    label: &str,
) -> Result<PathContainer, String> {
    let root = match path.first() {
        Some(YrsPathSegment::Key(root)) => root,
        Some(YrsPathSegment::Index(_)) => {
            return Err(format!("{label} path[0] must be a root-container key"));
        }
        None => return Err(format!("{label} path must not be empty")),
    };

    let root_kind = path
        .get(1)
        .map(YrsPathSegment::container_kind)
        .unwrap_or(expected);
    let mut current = resolve_root(txn, root, root_kind, label)?;

    for (index, segment) in path.iter().enumerate().skip(1) {
        let out = match (&current, segment) {
            (PathContainer::Map(map), YrsPathSegment::Key(key)) => map.get(txn, key),
            (PathContainer::Array(array), YrsPathSegment::Index(index)) => array.get(txn, *index),
            (PathContainer::Map(_), YrsPathSegment::Index(_)) => {
                return Err(format!("{label} path[{index}] must be a map key"));
            }
            (PathContainer::Array(_), YrsPathSegment::Key(_)) => {
                return Err(format!("{label} path[{index}] must be an array index"));
            }
        }
        .ok_or_else(|| format!("{label} path[{index}] does not resolve"))?;
        current = match out {
            Out::YMap(map) => PathContainer::Map(map),
            Out::YArray(array) => PathContainer::Array(array),
            other => {
                return Err(format!(
                    "{label} path[{index}] resolves to {}, not a container",
                    root_kind_label(&other)
                ));
            }
        };
    }

    if current.kind() == expected {
        Ok(current)
    } else {
        Err(format!(
            "{label} path does not resolve to a {}",
            expected.label()
        ))
    }
}

/// Resolve one root-level container, rejecting a kind the document contradicts.
///
/// Root branches are looked up by name, and the typed accessors (`get_map`,
/// `get_array`, `get_text`) cast the branch they find without consulting the
/// type it actually records. Reading a root under the wrong kind therefore
/// yields a usable handle onto a foreign branch, and writing through it
/// corrupts the branch while leaving the original view intact. So the requested
/// kind is checked against the recorded one here, before any caller can write.
///
/// A root whose kind has never been declared records no type at all: an encoded
/// document describes nested container kinds but not root ones, so a document
/// rehydrated from bytes carries untyped roots until the application declares
/// each kind by accessing it. Such a root is **rejected** rather than accepted
/// on trust, because there is no way to recover its kind from here. Inferring
/// it from the branch's content does not work: `yrs` decides the type of an
/// undeclared branch from raw block state that still counts deleted entries,
/// while every accessor reachable from outside the crate skips them, so a map
/// root whose entries had all been deleted reads as holding nothing and would
/// be accepted as any kind at all. The caller declares the kind instead, by
/// reading the root once through the matching accessor.
fn resolve_root<T: ReadTxn>(
    txn: &T,
    root: &str,
    kind: ContainerKind,
    label: &str,
) -> Result<PathContainer, String> {
    let missing = || format!("{label} root {} {root:?} does not exist", kind.label());
    match txn.get(root) {
        None => Err(missing()),
        Some(Out::YMap(map)) if kind == ContainerKind::Map => Ok(PathContainer::Map(map)),
        Some(Out::YArray(array)) if kind == ContainerKind::Array => Ok(PathContainer::Array(array)),
        Some(Out::UndefinedRef(_)) => Err(format!(
            "{label} root {root:?} has no declared kind; read it once through \
             the matching accessor before transferring into or out of it"
        )),
        Some(other) => Err(format!(
            "{label} root {root:?} has kind {}, not {}",
            root_kind_label(&other),
            kind.label()
        )),
    }
}

/// Human-readable kind of an already-resolved root branch, for error messages.
/// Never exposes the Rust type names the `Out` display impl would.
fn root_kind_label(out: &Out) -> &'static str {
    match out {
        Out::YMap(_) => "map",
        Out::YArray(_) => "array",
        Out::YText(_) => "text",
        Out::YXmlElement(_) => "xml element",
        Out::YXmlFragment(_) => "xml fragment",
        Out::YXmlText(_) => "xml text",
        Out::YDoc(_) => "subdocument",
        _ => "non-container value",
    }
}

impl YrsPathSegment {
    fn container_kind(&self) -> ContainerKind {
        match self {
            Self::Key(_) => ContainerKind::Map,
            Self::Index(_) => ContainerKind::Array,
        }
    }
}

impl PathContainer {
    fn kind(&self) -> ContainerKind {
        match self {
            Self::Map(_) => ContainerKind::Map,
            Self::Array(_) => ContainerKind::Array,
        }
    }
}

/// Deepest container nesting `transfer_entry_atomically` will copy.
///
/// Chosen well above any plausible authored document and well below the depth
/// that overflows a small thread stack, so the bound is reached only by
/// pathological input.
const MAX_TRANSFER_DEPTH: usize = 128;

/// Whether `out` can be deep-copied without overflowing the stack.
///
/// `Out::as_prelim` recurses once per nesting level, and nesting depth is
/// remote-influenced — a peer can ship an arbitrarily deep subtree through
/// `apply_update` — so a source has to be measured before it is copied. A stack
/// overflow aborts the process and cannot be caught, so this must not be the
/// thing that is wrong.
///
/// Exhaustiveness is necessary but not sufficient: it catches a *new* variant,
/// not an existing one classified as a leaf when it is not. Every arm below
/// that does not recurse is a claim about `as_prelim` that was checked against
/// its implementation. Note that enabling `yrs`'s `weak` feature anywhere in
/// the build graph adds a variant and so breaks this match — deliberately, and
/// this crate does not expose that feature.
///
/// The traversal mirrors `as_prelim`'s own recursion, and the match below is
/// deliberately exhaustive with no wildcard arm: if a future `yrs` adds an
/// `Out` variant, this stops compiling instead of silently letting an
/// unmeasured shape through.
fn validate_copy_depth<T: ReadTxn>(txn: &T, out: &Out, limit: usize) -> Result<(), String> {
    let too_deep = || Err(format!("source entry nests deeper than {limit} levels"));
    let mut pending = vec![(out.clone(), 1usize)];
    while let Some((current, depth)) = pending.pop() {
        if depth > limit {
            return too_deep();
        }
        match current {
            // `as_prelim` clones an `Any` without descending: its nested
            // values are shared by pointer, not rebuilt.
            Out::Any(_) => {}
            // A subdocument is cloned by reference, never copied into.
            Out::YDoc(_) => {}
            // Text is NOT a leaf. `as_prelim` maps the delta through
            // `diff.insert.as_prelim(txn)`, and an embed may be a container —
            // `Text::insert_embed` takes any prelim — so a chain of texts each
            // embedding the next recurses exactly as nested maps do.
            Out::YText(text) => {
                for diff in text.diff(txn, YChange::identity) {
                    pending.push((diff.insert, depth + 1));
                }
            }
            Out::YXmlText(text) => {
                for diff in text.diff(txn, YChange::identity) {
                    pending.push((diff.insert, depth + 1));
                }
                for (_, value) in text.attributes(txn) {
                    pending.push((value, depth + 1));
                }
            }
            Out::YMap(map) => {
                for (_, value) in map.iter(txn) {
                    pending.push((value, depth + 1));
                }
            }
            Out::YArray(array) => {
                for value in array.iter(txn) {
                    pending.push((value, depth + 1));
                }
            }
            Out::YXmlElement(element) => {
                for child in element.children(txn) {
                    pending.push((xml_child_out(child), depth + 1));
                }
                // Attributes are not leaves either. `insert_attribute` takes
                // any prelim, so an attribute may hold a container, and
                // `as_prelim` renders each one with `to_string` — which walks a
                // container value to JSON, one frame per level.
                for (_, value) in element.attributes(txn) {
                    pending.push((value, depth + 1));
                }
            }
            Out::YXmlFragment(fragment) => {
                for child in fragment.children(txn) {
                    pending.push((xml_child_out(child), depth + 1));
                }
            }
            // `as_prelim` types an undeclared branch by inspecting raw block
            // state this crate cannot reach, then recurses into whatever it
            // decided. Rather than guess at a shape and measure the wrong tree,
            // refuse: a nested branch of undeclared kind has no legitimate
            // source in a document this API is asked to move entries around in.
            Out::UndefinedRef(_) => {
                return Err("source entry contains a container of undeclared kind".to_owned());
            }
        }
    }
    Ok(())
}

/// Widen one xml child back to the `Out` the depth walk carries.
fn xml_child_out(child: XmlOut) -> Out {
    match child {
        XmlOut::Element(element) => Out::YXmlElement(element),
        XmlOut::Fragment(fragment) => Out::YXmlFragment(fragment),
        XmlOut::Text(text) => Out::YXmlText(text),
    }
}

impl ContainerKind {
    fn label(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Array => "array",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use yrs::types::ToJson;
    use yrs::{
        any, Any, GetString, MapPrelim, Text, TextPrelim, TransactionMut, Xml, XmlElementPrelim,
    };

    fn key(name: &str) -> YrsPathSegment {
        YrsPathSegment::Key(name.to_owned())
    }

    fn at(position: u32) -> YrsPathSegment {
        YrsPathSegment::Index(position)
    }

    fn array_slot(path: &[YrsPathSegment], index: u32) -> YrsTransferLocation {
        YrsTransferLocation::Array {
            path: path.to_vec(),
            index,
        }
    }

    fn map_slot(path: &[YrsPathSegment], key: &str) -> YrsTransferLocation {
        YrsTransferLocation::Map {
            path: path.to_vec(),
            key: key.to_owned(),
        }
    }

    fn array_at_map_key(
        parent_path: &[YrsPathSegment],
        key: &str,
        index: u32,
    ) -> YrsTransferLocation {
        YrsTransferLocation::ArrayAtMapKey {
            parent_path: parent_path.to_vec(),
            key: key.to_owned(),
            index,
        }
    }

    fn text(value: &str) -> In {
        In::from(value.to_owned())
    }

    /// Seed a root array with plain string entries.
    fn seed_list(doc: &YrsDoc, name: &str, values: &[&str]) -> ArrayRef {
        let array = doc.inner.get_or_insert_array(name);
        let mut txn = local_txn(&doc.inner);
        for value in values {
            array.push_back(&mut txn, text(value));
        }
        array
    }

    fn seed_map(doc: &YrsDoc, name: &str) -> MapRef {
        doc.inner.get_or_insert_map(name)
    }

    fn array_state(doc: &YrsDoc, name: &str) -> Any {
        let txn = doc.inner.transact();
        txn.get_array(name)
            .expect("root array must exist")
            .to_json(&txn)
    }

    fn map_state(doc: &YrsDoc, name: &str) -> Any {
        let txn = doc.inner.transact();
        txn.get_map(name)
            .expect("root map must exist")
            .to_json(&txn)
    }

    /// Run a transfer expected to succeed with a write.
    fn transfer(
        doc: &YrsDoc,
        mode: YrsTransferMode,
        source: YrsTransferLocation,
        target: YrsTransferLocation,
    ) {
        let outcome = doc
            .transfer_entry_atomically(mode, source, target)
            .expect("transfer was expected to succeed");
        assert!(
            outcome.changed_document(),
            "transfer was expected to write, got {outcome:?}"
        );
    }

    /// Run a transfer expected to be rejected, asserting the whole document is
    /// byte-identical afterwards, and return the error message.
    fn reject(
        doc: &YrsDoc,
        mode: YrsTransferMode,
        source: YrsTransferLocation,
        target: YrsTransferLocation,
    ) -> String {
        let before = doc.save();
        let error = doc
            .transfer_entry_atomically(mode, source, target)
            .expect_err("transfer was expected to be rejected");
        assert_eq!(
            doc.save(),
            before,
            "a rejected transfer must leave the document untouched"
        );
        error
    }

    /// Run a transfer expected to report `Unchanged`, asserting it wrote nothing.
    fn no_op(
        doc: &YrsDoc,
        mode: YrsTransferMode,
        source: YrsTransferLocation,
        target: YrsTransferLocation,
    ) {
        let before = doc.save();
        assert_eq!(
            doc.transfer_entry_atomically(mode, source, target),
            Ok(YrsTransferOutcome::Unchanged),
            "transfer was expected to be a no-op"
        );
        assert_eq!(
            doc.save(),
            before,
            "a no-op transfer must leave the document untouched"
        );
    }

    #[test]
    fn array_move_within_one_list_reorders_without_duplicating() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b", "c"]);

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_slot(&[key("list")], 3),
        );

        assert_eq!(array_state(&doc, "list"), any!(["b", "c", "a"]));
    }

    #[test]
    fn array_move_within_one_list_handles_backwards_moves() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b", "c"]);

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 2),
            array_slot(&[key("list")], 0),
        );

        assert_eq!(array_state(&doc, "list"), any!(["c", "a", "b"]));
    }

    #[test]
    fn array_copy_within_one_list_duplicates_the_entry() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);

        transfer(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("list")], 0),
            array_slot(&[key("list")], 2),
        );

        assert_eq!(array_state(&doc, "list"), any!(["a", "b", "a"]));
    }

    #[test]
    fn array_copy_to_an_adjacent_gap_still_writes() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);

        // The adjacent-gap short circuit is a Move-only rule; a Copy to the
        // same gap is a real duplication.
        transfer(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("list")], 0),
            array_slot(&[key("list")], 1),
        );

        assert_eq!(array_state(&doc, "list"), any!(["a", "a", "b"]));
    }

    #[test]
    fn array_move_across_lists_relocates_the_entry() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "left", &["a", "b"]);
        seed_list(&doc, "right", &["x"]);

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("left")], 0),
            array_slot(&[key("right")], 1),
        );

        assert_eq!(array_state(&doc, "left"), any!(["b"]));
        assert_eq!(array_state(&doc, "right"), any!(["x", "a"]));
    }

    #[test]
    fn array_copy_across_lists_leaves_the_source_in_place() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "left", &["a", "b"]);
        seed_list(&doc, "right", &[]);

        transfer(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("left")], 1),
            array_slot(&[key("right")], 0),
        );

        assert_eq!(array_state(&doc, "left"), any!(["a", "b"]));
        assert_eq!(array_state(&doc, "right"), any!(["b"]));
    }

    #[test]
    fn cross_list_move_carries_nested_container_content() {
        let doc = YrsDoc::new_empty();
        let left = doc.inner.get_or_insert_array("left");
        seed_list(&doc, "right", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let node = left.push_back(&mut txn, MapPrelim::default());
            node.insert(&mut txn, "title", text("card"));
            let tags = node.insert(&mut txn, "tags", ArrayPrelim::default());
            tags.push_back(&mut txn, text("t0"));
        }

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("left")], 0),
            array_slot(&[key("right")], 0),
        );

        assert_eq!(array_state(&doc, "left"), any!([]));
        assert_eq!(
            array_state(&doc, "right"),
            any!([{ "title": "card", "tags": ["t0"] }])
        );
    }

    #[test]
    fn map_slot_move_relocates_the_entry() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "from", text("payload"));
        }

        transfer(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "from"),
            map_slot(&[key("doc")], "to"),
        );

        assert_eq!(map_state(&doc, "doc"), any!({ "to": "payload" }));
    }

    #[test]
    fn map_slot_copy_leaves_the_source_in_place() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "from", text("payload"));
        }

        transfer(
            &doc,
            YrsTransferMode::Copy,
            map_slot(&[key("doc")], "from"),
            map_slot(&[key("doc")], "to"),
        );

        assert_eq!(
            map_state(&doc, "doc"),
            any!({ "from": "payload", "to": "payload" })
        );
    }

    #[test]
    fn array_to_map_and_map_to_array_transfers_are_supported() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        seed_map(&doc, "doc");

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            map_slot(&[key("doc")], "held"),
        );
        assert_eq!(array_state(&doc, "list"), any!([]));
        assert_eq!(map_state(&doc, "doc"), any!({ "held": "a" }));

        transfer(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "held"),
            array_slot(&[key("list")], 0),
        );
        assert_eq!(array_state(&doc, "list"), any!(["a"]));
        assert_eq!(map_state(&doc, "doc"), any!({}));
    }

    #[test]
    fn array_at_map_key_creates_the_destination_array() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);
        seed_map(&doc, "doc");

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_at_map_key(&[key("doc")], "kids", 0),
        );

        assert_eq!(array_state(&doc, "list"), any!(["b"]));
        assert_eq!(map_state(&doc, "doc"), any!({ "kids": ["a"] }));
    }

    #[test]
    fn array_at_map_key_appends_into_an_existing_array() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            let kids = root.insert(&mut txn, "kids", ArrayPrelim::default());
            kids.push_back(&mut txn, text("k0"));
        }

        transfer(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("list")], 0),
            array_at_map_key(&[key("doc")], "kids", 1),
        );

        assert_eq!(array_state(&doc, "list"), any!(["a"]));
        assert_eq!(map_state(&doc, "doc"), any!({ "kids": ["k0", "a"] }));
    }

    #[test]
    fn array_at_map_key_rejects_an_existing_non_array_without_writing() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "kids", text("not-a-list"));
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_at_map_key(&[key("doc")], "kids", 0),
        );

        assert!(
            error.contains("target map key \"kids\" resolves to") && error.contains("not an array"),
            "unexpected error: {error}"
        );
        assert_eq!(map_state(&doc, "doc"), any!({ "kids": "not-a-list" }));
    }

    #[test]
    fn array_at_map_key_rejects_an_out_of_range_index_without_writing() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        seed_map(&doc, "doc");

        // Absent key: the array does not exist yet, so only gap 0 is valid.
        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_at_map_key(&[key("doc")], "kids", 1),
        );
        assert!(
            error.contains("outside insertion range 0..=0"),
            "unexpected error: {error}"
        );
        assert_eq!(map_state(&doc, "doc"), any!({}));
    }

    #[test]
    fn array_at_map_key_is_rejected_as_a_source() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &[]);
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            let kids = root.insert(&mut txn, "kids", ArrayPrelim::default());
            kids.push_back(&mut txn, text("k0"));
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_at_map_key(&[key("doc")], "kids", 0),
            array_slot(&[key("list")], 0),
        );

        assert_eq!(error, "array-at-map-key is a destination-only location");
    }

    #[test]
    fn same_list_move_to_either_adjacent_gap_is_a_no_op() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b", "c"]);

        // The gap before the entry.
        no_op(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 1),
            array_slot(&[key("list")], 1),
        );
        // The gap after the entry.
        no_op(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 1),
            array_slot(&[key("list")], 2),
        );

        assert_eq!(array_state(&doc, "list"), any!(["a", "b", "c"]));
    }

    #[test]
    fn same_list_move_of_the_last_entry_to_the_end_gap_is_a_no_op() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);

        no_op(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 1),
            array_slot(&[key("list")], 2),
        );

        assert_eq!(array_state(&doc, "list"), any!(["a", "b"]));
    }

    #[test]
    fn an_array_source_and_an_array_at_map_key_target_can_name_the_same_list() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            let kids = root.insert(&mut txn, "kids", ArrayPrelim::default());
            for value in ["a", "b", "c"] {
                kids.push_back(&mut txn, text(value));
            }
        }
        let list_path = [key("doc"), key("kids")];

        // Both locations resolve to the same `ArrayRef`, so the same-list
        // rules apply across the two location variants.
        no_op(
            &doc,
            YrsTransferMode::Move,
            array_slot(&list_path, 1),
            array_at_map_key(&[key("doc")], "kids", 2),
        );

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&list_path, 0),
            array_at_map_key(&[key("doc")], "kids", 3),
        );

        assert_eq!(map_state(&doc, "doc"), any!({ "kids": ["b", "c", "a"] }));
    }

    #[test]
    fn a_move_into_a_sibling_that_precedes_the_source_relocates_correctly() {
        let doc = YrsDoc::new_empty();
        let list = doc.inner.get_or_insert_array("list");
        {
            let mut txn = local_txn(&doc.inner);
            list.push_back(&mut txn, ArrayPrelim::default());
            list.push_back(&mut txn, text("a"));
        }

        // The destination lives at an index below the source, so the source
        // removal shifts it. The resolved refs must survive that.
        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 1),
            array_slot(&[key("list"), at(0)], 0),
        );

        assert_eq!(array_state(&doc, "list"), any!([["a"]]));
    }

    #[test]
    fn same_map_entry_move_is_a_no_op_while_copy_is_rejected() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "slot", text("payload"));
        }

        no_op(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "slot"),
            map_slot(&[key("doc")], "slot"),
        );

        let error = reject(
            &doc,
            YrsTransferMode::Copy,
            map_slot(&[key("doc")], "slot"),
            map_slot(&[key("doc")], "slot"),
        );
        assert_eq!(error, "target map key \"slot\" is already occupied");
    }

    #[test]
    fn out_of_range_source_index_is_rejected_without_writing() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);

        // The end gap is a valid destination but never a valid source.
        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 2),
            array_slot(&[key("list")], 0),
        );

        assert_eq!(error, "source array index 2 is outside entry range 0..2");
        assert_eq!(array_state(&doc, "list"), any!(["a", "b"]));
    }

    #[test]
    fn out_of_range_target_index_is_rejected_without_writing() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "left", &["a"]);
        seed_list(&doc, "right", &["x"]);

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("left")], 0),
            array_slot(&[key("right")], 2),
        );

        assert_eq!(
            error,
            "target array index 2 is outside insertion range 0..=1"
        );
        assert_eq!(array_state(&doc, "left"), any!(["a"]));
        assert_eq!(array_state(&doc, "right"), any!(["x"]));
    }

    #[test]
    fn missing_source_entries_are_rejected_without_writing() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &[]);
        seed_map(&doc, "doc");

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "absent"),
            array_slot(&[key("list")], 0),
        );
        assert_eq!(error, "source map key \"absent\" does not exist");

        let error = reject(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("list")], 0),
            map_slot(&[key("doc")], "landing"),
        );
        assert_eq!(error, "source array index 0 is outside entry range 0..0");
    }

    #[test]
    fn occupied_target_map_key_is_rejected_without_writing() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "slot", text("existing"));
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            map_slot(&[key("doc")], "slot"),
        );

        assert_eq!(error, "target map key \"slot\" is already occupied");
        assert_eq!(array_state(&doc, "list"), any!(["a"]));
        assert_eq!(map_state(&doc, "doc"), any!({ "slot": "existing" }));
    }

    /// `doc.sections` is a list of `{ items: [...] }` nodes.
    fn seed_sections(doc: &YrsDoc, first: &[&str], second: &[&str]) {
        let root = doc.inner.get_or_insert_map("doc");
        let mut txn = local_txn(&doc.inner);
        let sections = root.insert(&mut txn, "sections", ArrayPrelim::default());
        for values in [first, second] {
            let node = sections.push_back(&mut txn, MapPrelim::default());
            let items = node.insert(&mut txn, "items", ArrayPrelim::default());
            for value in values {
                items.push_back(&mut txn, text(value));
            }
        }
    }

    #[test]
    fn deep_paths_move_between_nested_lists() {
        let doc = YrsDoc::new_empty();
        seed_sections(&doc, &["i0", "i1"], &["j0"]);

        let source_path = [key("doc"), key("sections"), at(0), key("items")];
        let target_path = [key("doc"), key("sections"), at(1), key("items")];
        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&source_path, 0),
            array_slot(&target_path, 0),
        );

        assert_eq!(
            map_state(&doc, "doc"),
            any!({
                "sections": [
                    { "items": ["i1"] },
                    { "items": ["i0", "j0"] }
                ]
            })
        );
    }

    #[test]
    fn deep_paths_copy_into_a_nested_map_slot() {
        let doc = YrsDoc::new_empty();
        seed_sections(&doc, &["i0"], &[]);

        let source_path = [key("doc"), key("sections"), at(0), key("items")];
        let target_path = [key("doc"), key("sections"), at(1)];
        transfer(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&source_path, 0),
            map_slot(&target_path, "label"),
        );

        assert_eq!(
            map_state(&doc, "doc"),
            any!({
                "sections": [
                    { "items": ["i0"] },
                    { "items": [], "label": "i0" }
                ]
            })
        );
    }

    #[test]
    fn copy_produces_an_independent_deep_clone() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &[]);
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            let template = root.insert(&mut txn, "template", MapPrelim::default());
            template.insert(&mut txn, "title", text("T"));
            template.insert(&mut txn, "body", TextPrelim::new("hello"));
            let tags = template.insert(&mut txn, "tags", ArrayPrelim::default());
            tags.push_back(&mut txn, text("x"));
        }

        transfer(
            &doc,
            YrsTransferMode::Copy,
            map_slot(&[key("doc")], "template"),
            array_slot(&[key("list")], 0),
        );

        // Mutate every container kind inside the clone.
        {
            let mut txn = local_txn(&doc.inner);
            let list = txn.get_array("list").expect("root array must exist");
            let Some(Out::YMap(clone)) = list.get(&txn, 0) else {
                panic!("the clone must be a map");
            };
            clone.insert(&mut txn, "title", text("changed"));
            let Some(Out::YArray(tags)) = clone.get(&txn, "tags") else {
                panic!("the clone must carry a tags array");
            };
            tags.push_back(&mut txn, text("y"));
            let Some(Out::YText(body)) = clone.get(&txn, "body") else {
                panic!("the clone must carry a body text");
            };
            body.insert(&mut txn, 0, "X");
        }

        assert_eq!(
            map_state(&doc, "doc"),
            any!({
                "template": { "title": "T", "body": "hello", "tags": ["x"] }
            }),
            "mutating the clone must not touch the original"
        );
        assert_eq!(
            array_state(&doc, "list"),
            any!([{ "title": "changed", "body": "Xhello", "tags": ["x", "y"] }])
        );
    }

    #[test]
    fn moving_an_array_entry_into_its_own_descendant_is_rejected() {
        let doc = YrsDoc::new_empty();
        let list = doc.inner.get_or_insert_array("list");
        {
            let mut txn = local_txn(&doc.inner);
            let nested = list.push_back(&mut txn, ArrayPrelim::default());
            nested.push_back(&mut txn, text("inner"));
            list.push_back(&mut txn, text("tail"));
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_slot(&[key("list"), at(0)], 0),
        );

        assert_eq!(error, "target path descends into the source entry");
        assert_eq!(array_state(&doc, "list"), any!([["inner"], "tail"]));
    }

    #[test]
    fn moving_a_map_entry_into_its_own_descendant_is_rejected() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            let branch = root.insert(&mut txn, "branch", MapPrelim::default());
            branch.insert(&mut txn, "leaf", MapPrelim::default());
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "branch"),
            map_slot(&[key("doc"), key("branch"), key("leaf")], "held"),
        );

        assert_eq!(error, "target path descends into the source entry");
        assert_eq!(map_state(&doc, "doc"), any!({ "branch": { "leaf": {} } }));
    }

    #[test]
    fn copying_an_entry_into_its_own_descendant_is_rejected() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            let branch = root.insert(&mut txn, "branch", MapPrelim::default());
            branch.insert(&mut txn, "kids", ArrayPrelim::default());
        }

        // The ancestry guard runs before the mode split, so Copy is rejected
        // too even though a clone would not be self-referential.
        let error = reject(
            &doc,
            YrsTransferMode::Copy,
            map_slot(&[key("doc")], "branch"),
            array_at_map_key(&[key("doc"), key("branch")], "kids", 0),
        );

        assert_eq!(error, "target path descends into the source entry");
    }

    #[test]
    fn moving_a_map_entry_onto_the_array_it_names_is_rejected() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "kids", ArrayPrelim::default());
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "kids"),
            array_at_map_key(&[key("doc")], "kids", 0),
        );

        assert_eq!(error, "target path descends into the source entry");
    }

    #[test]
    fn sibling_subtrees_are_not_treated_as_descendants() {
        let doc = YrsDoc::new_empty();
        let root = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            root.insert(&mut txn, "branch", text("payload"));
            root.insert(&mut txn, "other", MapPrelim::default());
        }

        transfer(
            &doc,
            YrsTransferMode::Move,
            map_slot(&[key("doc")], "branch"),
            map_slot(&[key("doc"), key("other")], "held"),
        );

        assert_eq!(
            map_state(&doc, "doc"),
            any!({ "other": { "held": "payload" } })
        );
    }

    #[test]
    fn a_neighbouring_index_sharing_the_source_prefix_is_not_a_descendant() {
        let doc = YrsDoc::new_empty();
        let list = doc.inner.get_or_insert_array("list");
        {
            let mut txn = local_txn(&doc.inner);
            list.push_back(&mut txn, text("a"));
            list.push_back(&mut txn, ArrayPrelim::default());
        }

        // `list[1]` shares the `["list"]` prefix but is not inside `list[0]`.
        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_slot(&[key("list"), at(1)], 0),
        );

        assert_eq!(array_state(&doc, "list"), any!([["a"]]));
    }

    #[test]
    fn malformed_paths_are_rejected_before_any_write() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        seed_sections(&doc, &["i0"], &[]);

        let cases: Vec<(YrsTransferLocation, YrsTransferLocation, &str)> = vec![
            (
                array_slot(&[], 0),
                array_slot(&[key("list")], 0),
                "source path must not be empty",
            ),
            (
                array_slot(&[at(0)], 0),
                array_slot(&[key("list")], 0),
                "source path[0] must be a root-container key",
            ),
            (
                array_slot(&[key("list")], 0),
                array_slot(&[], 0),
                "target path must not be empty",
            ),
            (
                array_slot(&[key("missing")], 0),
                array_slot(&[key("list")], 0),
                "source root array \"missing\" does not exist",
            ),
            (
                map_slot(&[key("missing"), key("nested")], "k"),
                array_slot(&[key("list")], 0),
                "source root map \"missing\" does not exist",
            ),
            (
                array_slot(&[key("doc"), key("sections"), at(0), at(0)], 0),
                array_slot(&[key("list")], 0),
                "source path[3] must be a map key",
            ),
            (
                array_slot(&[key("doc"), key("sections"), key("nope")], 0),
                array_slot(&[key("list")], 0),
                "source path[2] must be an array index",
            ),
            (
                array_slot(&[key("doc"), key("sections"), at(9)], 0),
                array_slot(&[key("list")], 0),
                "source path[2] does not resolve",
            ),
            (
                array_slot(
                    &[key("doc"), key("sections"), at(0), key("items"), at(0)],
                    0,
                ),
                array_slot(&[key("list")], 0),
                // The offending value's kind is named without exposing the
                // underlying Rust type name.
                "source path[4] resolves to non-container value, not a container",
            ),
            (
                map_slot(&[key("doc"), key("sections")], "k"),
                array_slot(&[key("list")], 0),
                "source path does not resolve to a map",
            ),
            (
                array_slot(&[key("list")], 0),
                array_slot(&[key("doc"), key("sections"), at(0)], 0),
                // Verbatim: the message interpolates a bare kind label, so it
                // reads "a array" rather than "an array".
                "target path does not resolve to a array",
            ),
        ];

        for (source, target, expected) in cases {
            let error = reject(&doc, YrsTransferMode::Move, source, target);
            assert_eq!(error, expected);
        }
    }

    #[test]
    fn a_root_addressed_under_the_wrong_kind_is_rejected() {
        // Roots are looked up by name and the typed accessors cast whatever
        // branch they find, so an array root also hands back a usable map
        // handle. Writing through that handle would leave the same root holding
        // both list items and map entries, with each reader seeing only its own
        // half. The recorded kind is validated instead.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);

        let error = reject(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("list")], 0),
            map_slot(&[key("list")], "slot"),
        );

        assert_eq!(error, "target root \"list\" has kind array, not map");
        assert_eq!(array_state(&doc, "list"), any!(["a"]));
    }

    #[test]
    fn a_root_text_cannot_be_addressed_as_a_container() {
        // A text root read as an array yields a handle whose "entries" are the
        // text's own chunks: an array-addressed write destroys characters while
        // `get_string` keeps returning the original value.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        let note = doc.inner.get_or_insert_text("note");
        {
            let mut txn = local_txn(&doc.inner);
            note.insert(&mut txn, 0, "hello world");
        }

        let cases: Vec<(YrsTransferLocation, YrsTransferLocation, &str)> = vec![
            (
                array_slot(&[key("list")], 0),
                array_slot(&[key("note")], 0),
                "target root \"note\" has kind text, not array",
            ),
            (
                array_slot(&[key("note")], 0),
                array_slot(&[key("list")], 1),
                "source root \"note\" has kind text, not array",
            ),
            (
                array_slot(&[key("list")], 0),
                map_slot(&[key("note")], "slot"),
                "target root \"note\" has kind text, not map",
            ),
            (
                map_slot(&[key("note")], "slot"),
                array_slot(&[key("list")], 1),
                "source root \"note\" has kind text, not map",
            ),
        ];

        for (source, target, expected) in cases {
            let error = reject(&doc, YrsTransferMode::Move, source, target);
            assert_eq!(error, expected);
        }

        assert_eq!(note.get_string(&doc.inner.transact()), "hello world");
        assert_eq!(array_state(&doc, "list"), any!(["a"]));
    }

    #[test]
    fn a_nested_root_step_is_validated_against_the_recorded_kind() {
        // The root check also covers paths that only pass through the root: a
        // map root walked as if its first step were an array index.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a"]);
        let template = seed_map(&doc, "doc");
        {
            let mut txn = local_txn(&doc.inner);
            template.insert(&mut txn, "kids", ArrayPrelim::default());
        }

        let error = reject(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_slot(&[key("doc"), at(0)], 0),
        );

        assert_eq!(error, "target root \"doc\" has kind map, not array");
    }

    #[test]
    fn a_root_whose_kind_was_never_declared_is_rejected() {
        // An encoded document records nested container kinds but not root ones,
        // so every root of a rehydrated document is undeclared. A root's kind
        // cannot be recovered from outside `yrs` — see
        // `an_emptied_root_reads_the_same_as_an_empty_one` — so a transfer
        // refuses rather than guessing, and says what to do instead.
        let source_doc = YrsDoc::new_empty();
        seed_list(&source_doc, "list", &["a"]);
        let template = seed_map(&source_doc, "doc");
        {
            let mut txn = local_txn(&source_doc.inner);
            template.insert(&mut txn, "seed", "x");
        }

        let doc = YrsDoc::from_bytes(source_doc.save()).expect("saved bytes must reload");
        let before = doc.save();

        let error = doc
            .transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("list")], 0),
                map_slot(&[key("doc")], "moved"),
            )
            .expect_err("an undeclared root must not be addressed");
        assert!(error.contains("no declared kind"), "{error}");
        assert_eq!(doc.save(), before, "the rejection left the document alone");

        // Reading each root once through its accessor is the whole remedy, and
        // it is what an application that renders the document does anyway.
        let _ = doc.get_array("list".to_owned());
        let _ = doc.get_map("doc".to_owned());
        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            map_slot(&[key("doc")], "moved"),
        );

        assert_eq!(array_state(&doc, "list"), any!([]));
        assert_eq!(map_state(&doc, "doc"), any!({ "seed": "x", "moved": "a" }));
    }

    #[test]
    fn a_failed_transfer_leaves_the_undo_visible_state_untouched() {
        // A stronger form of the per-case atomicity assertion: run every
        // rejecting shape against one document and compare the full encoded
        // state to the pristine snapshot each time.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);
        seed_map(&doc, "doc");
        let pristine = doc.save();

        let cases: Vec<(YrsTransferMode, YrsTransferLocation, YrsTransferLocation)> = vec![
            (
                YrsTransferMode::Move,
                array_slot(&[key("list")], 5),
                map_slot(&[key("doc")], "k"),
            ),
            (
                YrsTransferMode::Move,
                array_slot(&[key("list")], 0),
                array_slot(&[key("list")], 9),
            ),
            (
                YrsTransferMode::Copy,
                map_slot(&[key("doc")], "absent"),
                array_slot(&[key("list")], 0),
            ),
            (
                YrsTransferMode::Move,
                array_at_map_key(&[key("doc")], "kids", 0),
                array_slot(&[key("list")], 0),
            ),
            (
                YrsTransferMode::Move,
                array_slot(&[key("list")], 0),
                array_slot(&[key("list"), at(0)], 0),
            ),
        ];

        for (mode, source, target) in cases {
            assert!(doc.transfer_entry_atomically(mode, source, target).is_err());
            assert_eq!(doc.save(), pristine);
        }

        assert_eq!(array_state(&doc, "list"), any!(["a", "b"]));
        assert_eq!(map_state(&doc, "doc"), any!({}));
    }

    #[test]
    fn a_successful_transfer_is_visible_to_a_synced_peer() {
        // The write must land in one committed transaction that a peer can
        // consume, not in per-step transactions.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "left", &["a", "b"]);
        seed_list(&doc, "right", &[]);
        let peer = YrsDoc::from_bytes(doc.save()).expect("peer must hydrate");

        let before = doc.get_state_vector();
        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("left")], 0),
            array_slot(&[key("right")], 0),
        );
        let diff = doc
            .encode_state_as_update(Some(before))
            .expect("diff must encode");
        peer.apply_update(diff).expect("peer must apply the diff");

        assert_eq!(array_state(&peer, "left"), any!(["b"]));
        assert_eq!(array_state(&peer, "right"), any!(["a"]));
    }

    #[test]
    fn transfers_are_tagged_with_the_local_origin() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "list", &["a", "b"]);
        let seen: std::sync::Arc<Mutex<Vec<Option<Vec<u8>>>>> = Default::default();
        let sink = seen.clone();
        let _sub = doc
            .inner
            .observe_transaction_cleanup(move |txn: &TransactionMut<'_>, _| {
                sink.lock()
                    .unwrap()
                    .push(txn.origin().map(|origin| origin.as_ref().to_vec()));
            })
            .expect("observer must attach");

        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            array_slot(&[key("list")], 2),
        );

        assert_eq!(
            seen.lock().unwrap().as_slice(),
            [Some(crate::api::origin::YRS_DART_LOCAL_ORIGIN.to_vec())]
        );
    }

    #[test]
    fn a_same_list_move_reports_a_crdt_move_and_a_cross_list_move_reports_a_reparent() {
        // The distinction I-1 exists to surface: one public method, two
        // different collaborative semantics, decided by whether the parent
        // changes. Callers must be able to tell which one ran.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "a", &["x", "y", "z"]);
        seed_list(&doc, "b", &["q"]);

        assert_eq!(
            doc.transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("a")], 0),
                array_slot(&[key("a")], 3),
            ),
            Ok(YrsTransferOutcome::Moved),
            "a move inside one list is a real CRDT move"
        );

        assert_eq!(
            doc.transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("a")], 0),
                array_slot(&[key("b")], 0),
            ),
            Ok(YrsTransferOutcome::Reparented),
            "a move between lists cannot be a CRDT move"
        );

        assert_eq!(
            doc.transfer_entry_atomically(
                YrsTransferMode::Copy,
                array_slot(&[key("a")], 0),
                array_slot(&[key("b")], 0),
            ),
            Ok(YrsTransferOutcome::Copied),
            "a copy never removes the source"
        );
    }

    #[test]
    fn only_the_unchanged_outcome_reports_no_document_change() {
        assert!(!YrsTransferOutcome::Unchanged.changed_document());
        assert!(YrsTransferOutcome::Moved.changed_document());
        assert!(YrsTransferOutcome::Reparented.changed_document());
        assert!(YrsTransferOutcome::Copied.changed_document());
    }

    #[test]
    fn a_reparent_discards_a_concurrent_remote_edit_to_the_moved_subtree() {
        // Documents the cost of the reparent path so a future change cannot
        // quietly alter it. This is exactly what `Reparented` warns about.
        let local = YrsDoc::new_empty();
        let a = local.inner.get_or_insert_array("a");
        let _b = local.inner.get_or_insert_array("b");
        {
            let mut txn = local_txn(&local.inner);
            let node = a.push_back(&mut txn, MapPrelim::default());
            node.insert(&mut txn, "k", "v");
        }

        // A peer starts from the same state.
        let remote = YrsDoc::new_empty();
        remote
            .apply_update(
                local
                    .inner
                    .transact()
                    .encode_state_as_update_v1(&Default::default()),
            )
            .expect("peer syncs");

        // Concurrently: the peer edits the entry, we reparent it.
        {
            let r_a = remote.inner.get_or_insert_array("a");
            let mut txn = local_txn(&remote.inner);
            let Some(Out::YMap(node)) = r_a.get(&txn, 0) else {
                panic!("peer sees the map entry");
            };
            node.insert(&mut txn, "peer", "edit");
        }
        assert_eq!(
            local.transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("a")], 0),
                array_slot(&[key("b")], 0),
            ),
            Ok(YrsTransferOutcome::Reparented)
        );

        // Converge both ways.
        let to_remote = local
            .inner
            .transact()
            .encode_state_as_update_v1(&remote.inner.transact().state_vector());
        let to_local = remote
            .inner
            .transact()
            .encode_state_as_update_v1(&local.inner.transact().state_vector());
        remote.apply_update(to_remote).expect("apply");
        local.apply_update(to_local).expect("apply");

        // The peer's concurrent edit is gone on both sides. If a future change
        // preserves it, that is an improvement — update this test deliberately.
        let converged = array_state(&local, "b");
        assert_eq!(converged, array_state(&remote, "b"), "peers converge");
        assert_eq!(
            converged,
            any!([{ "k": "v" }]),
            "the reparented subtree is a copy of the state the mover saw"
        );
    }

    #[test]
    fn an_emptied_root_reads_the_same_as_an_empty_one() {
        // Why an undeclared root is refused outright instead of being typed
        // from its content. `yrs` types an undeclared branch from raw block
        // state, which still records deleted entries; every accessor reachable
        // from outside the crate skips them. A map root whose entries have all
        // been deleted therefore reads as holding nothing, and a content probe
        // would have concluded "nothing contradicts this" and accepted the
        // array request below — the original corruption, on exactly the path a
        // collaborative editor takes.
        let source = YrsDoc::new_empty();
        seed_list(&source, "list", &["a"]);
        let cfg = seed_map(&source, "cfg");
        {
            let mut txn = local_txn(&source.inner);
            cfg.insert(&mut txn, "k", "v");
        }
        {
            let mut txn = local_txn(&source.inner);
            cfg.remove(&mut txn, "k");
        }
        let doc = YrsDoc::from_bytes(source.save()).expect("reload");
        let before = doc.save();

        // The emptied map root is publicly indistinguishable from an array
        // root: no entries, no length, no text.
        {
            let txn = doc.inner.transact();
            assert_eq!(txn.get_map("cfg").map_or(0, |map| map.len(&txn)), 0);
            assert_eq!(txn.get_array("cfg").map_or(0, |array| array.len(&txn)), 0);
        }

        let error = doc
            .transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("list")], 0),
                array_slot(&[key("cfg")], 0),
            )
            .expect_err("an undeclared root must be refused however empty it reads");
        assert!(error.contains("no declared kind"), "{error}");
        assert_eq!(doc.save(), before, "the rejection left the document alone");

        // Declared, the recorded kind does the work the content never could.
        let _ = doc.get_array("list".to_owned());
        let _ = doc.get_map("cfg".to_owned());
        let declared = doc.save();
        let error = doc
            .transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("list")], 0),
                array_slot(&[key("cfg")], 0),
            )
            .expect_err("a map root must not be addressed as an array");
        assert!(error.contains("has kind map, not array"), "{error}");
        assert_eq!(doc.save(), declared, "still untouched");
    }

    #[test]
    fn a_declared_text_root_cannot_be_addressed_as_a_container() {
        // The destructive shape from the original defect: a text root
        // addressed as a list was silently cast and written through while
        // still reporting its original string. Once kinds are declared the
        // recorded kind rejects it and the text is left alone.
        let source = YrsDoc::new_empty();
        seed_list(&source, "list", &["a"]);
        // Resolve every container BEFORE opening a transaction: the
        // `get_or_insert_*` accessors take their own, and deadlock against a
        // live one.
        let notes = source.inner.get_or_insert_text("notes");
        {
            let mut txn = local_txn(&source.inner);
            notes.insert(&mut txn, 0, "hello world");
        }
        let doc = YrsDoc::from_bytes(source.save()).expect("reload");
        let _ = doc.get_array("list".to_owned());
        let reloaded_notes = doc.inner.get_or_insert_text("notes");
        let before = doc.save();

        for target in [
            array_slot(&[key("notes")], 0),
            map_slot(&[key("notes")], "slot"),
        ] {
            let error = doc
                .transfer_entry_atomically(
                    YrsTransferMode::Move,
                    array_slot(&[key("list")], 0),
                    target,
                )
                .expect_err("a text root is neither a list nor a map");
            assert!(error.contains("has kind text"), "{error}");
        }

        assert_eq!(
            doc.save(),
            before,
            "every rejection left the document alone"
        );
        let txn = doc.inner.transact();
        assert_eq!(reloaded_notes.get_string(&txn), "hello world");
    }

    #[test]
    fn an_empty_root_does_not_survive_a_round_trip_at_all() {
        // An empty root is never encoded, so after a reload it does not exist
        // and is rejected on existence rather than on kind. Pinned so the two
        // rejections stay distinguishable.
        let source = YrsDoc::new_empty();
        seed_list(&source, "list", &["a"]);
        seed_map(&source, "empty");
        let doc = YrsDoc::from_bytes(source.save()).expect("reload");
        let _ = doc.get_array("list".to_owned());

        let err = doc
            .transfer_entry_atomically(
                YrsTransferMode::Move,
                array_slot(&[key("list")], 0),
                map_slot(&[key("empty")], "slot"),
            )
            .expect_err("an unencoded root cannot be a target");
        assert!(err.contains("does not exist"), "{err}");

        // Declaring it locally brings it back, and then it behaves normally.
        let _ = doc.get_map("empty".to_owned());
        transfer(
            &doc,
            YrsTransferMode::Move,
            array_slot(&[key("list")], 0),
            map_slot(&[key("empty")], "slot"),
        );
        assert_eq!(map_state(&doc, "empty"), any!({ "slot": "a" }));
    }

    #[test]
    fn a_pathologically_deep_source_is_rejected_rather_than_crashing() {
        // Before the depth guard this aborted the whole process with a stack
        // overflow — uncatchable from Dart, so a hard crash in a host app.
        // Nesting is remote-influenced, so it must be an ordinary rejection.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &["z"]);
        let deep = seed_list(&doc, "deep", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let mut node = deep.push_back(&mut txn, MapPrelim::default());
            for _ in 0..(MAX_TRANSFER_DEPTH + 50) {
                node = node.insert(&mut txn, "child", MapPrelim::default());
            }
        }
        let before = doc.save();

        let err = doc
            .transfer_entry_atomically(
                YrsTransferMode::Copy,
                array_slot(&[key("deep")], 0),
                array_slot(&[key("dst")], 0),
            )
            .expect_err("a subtree past the depth bound is rejected");
        assert!(err.contains("nests deeper than"), "{err}");
        assert_eq!(doc.save(), before, "a rejected transfer writes nothing");
    }

    #[test]
    fn a_pathologically_deep_xml_source_is_rejected_too() {
        // `as_prelim` recurses through xml children exactly as it does through
        // map and array values, so a guard that measured only maps and arrays
        // let an xml chain of any depth through unmeasured. The guard now walks
        // the same shapes `as_prelim` does, and its match is exhaustive so a
        // new `Out` variant cannot quietly reopen the hole. (Whether a given
        // depth overflows is machine- and stack-dependent; what is asserted
        // here is that the bound is applied at all.)
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &["z"]);
        let deep = seed_list(&doc, "deep", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let mut node = deep.push_back(&mut txn, XmlElementPrelim::empty("node"));
            for _ in 0..(MAX_TRANSFER_DEPTH + 50) {
                node = node.insert(&mut txn, 0, XmlElementPrelim::empty("node"));
            }
        }
        let before = doc.save();

        let err = doc
            .transfer_entry_atomically(
                YrsTransferMode::Copy,
                array_slot(&[key("deep")], 0),
                array_slot(&[key("dst")], 0),
            )
            .expect_err("a deep xml subtree past the depth bound is rejected");
        assert!(err.contains("nests deeper than"), "{err}");
        assert_eq!(doc.save(), before, "a rejected transfer writes nothing");
    }

    #[test]
    fn a_pathologically_deep_text_embed_chain_is_rejected_too() {
        // Text is not a leaf. `as_prelim` maps a text delta through
        // `diff.insert.as_prelim(txn)`, and `Text::insert_embed` accepts any
        // prelim, so a chain of texts each embedding the next recurses exactly
        // as nested maps do. Treating text as terminal left the crash this
        // guard exists to prevent fully reachable, and reproducibly aborted the
        // process on a small stack.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &["z"]);
        let deep = seed_list(&doc, "deep", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let mut node = deep.push_back(&mut txn, ArrayPrelim::default());
            for _ in 0..(MAX_TRANSFER_DEPTH + 50) {
                let text = node.insert(&mut txn, 0, TextPrelim::new("x"));
                node = text.insert_embed(&mut txn, 0, ArrayPrelim::default());
            }
        }
        let before = doc.save();

        let err = doc
            .transfer_entry_atomically(
                YrsTransferMode::Copy,
                array_slot(&[key("deep")], 0),
                array_slot(&[key("dst")], 0),
            )
            .expect_err("a deep text-embed chain past the depth bound is rejected");
        assert!(err.contains("nests deeper than"), "{err}");
        assert_eq!(doc.save(), before, "a rejected transfer writes nothing");
    }

    #[test]
    fn an_ordinary_text_embed_still_transfers() {
        // The guard must not break the shape it sits in front of.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &[]);
        let src = seed_list(&doc, "src", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let text = src.push_back(&mut txn, TextPrelim::new("hello"));
            text.insert_embed(&mut txn, 0, ArrayPrelim::default());
        }

        doc.transfer_entry_atomically(
            YrsTransferMode::Copy,
            array_slot(&[key("src")], 0),
            array_slot(&[key("dst")], 0),
        )
        .expect("an ordinary text embed copies");
        assert_eq!(doc.get_array("dst".to_owned()).length(), 1);
    }

    #[test]
    fn a_pathologically_deep_xml_attribute_chain_is_rejected_too() {
        // An xml attribute is not a leaf either. `insert_attribute` takes any
        // prelim, so an attribute can hold a container, and `as_prelim` renders
        // each attribute with `to_string`, which walks a container value to
        // JSON one frame per level. Walking only children left this reachable:
        // the guard returned `Ok` and the copy it approved aborted the process
        // on a small stack.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &["z"]);
        let deep = seed_list(&doc, "deep", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let element = deep.push_back(&mut txn, XmlElementPrelim::empty("node"));
            let mut node = element.insert_attribute(&mut txn, "child", MapPrelim::default());
            for _ in 0..(MAX_TRANSFER_DEPTH + 50) {
                node = node.insert(&mut txn, "child", MapPrelim::default());
            }
        }
        let before = doc.save();

        let err = doc
            .transfer_entry_atomically(
                YrsTransferMode::Copy,
                array_slot(&[key("deep")], 0),
                array_slot(&[key("dst")], 0),
            )
            .expect_err("a deep attribute chain past the depth bound is rejected");
        assert!(err.contains("nests deeper than"), "{err}");
        assert_eq!(doc.save(), before, "a rejected transfer writes nothing");
    }

    #[test]
    fn an_ordinary_xml_attribute_still_transfers() {
        // The guard must not break the shape it sits in front of.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &[]);
        let src = seed_list(&doc, "src", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let element = src.push_back(&mut txn, XmlElementPrelim::empty("node"));
            element.insert_attribute(&mut txn, "plain", "value".to_owned());
            let nested = element.insert_attribute(&mut txn, "held", MapPrelim::default());
            nested.insert(&mut txn, "k", "v");
        }

        doc.transfer_entry_atomically(
            YrsTransferMode::Copy,
            array_slot(&[key("src")], 0),
            array_slot(&[key("dst")], 0),
        )
        .expect("an ordinary attributed element copies");
        assert_eq!(doc.get_array("dst".to_owned()).length(), 1);
    }

    #[test]
    fn an_ordinary_xml_source_still_transfers() {
        // The guard must not break the shape it sits in front of.
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &[]);
        let src = seed_list(&doc, "src", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let node = src.push_back(&mut txn, XmlElementPrelim::empty("node"));
            node.insert(&mut txn, 0, XmlElementPrelim::empty("child"));
        }

        doc.transfer_entry_atomically(
            YrsTransferMode::Copy,
            array_slot(&[key("src")], 0),
            array_slot(&[key("dst")], 0),
        )
        .expect("an ordinary xml subtree copies");
        assert_eq!(doc.get_array("dst".to_owned()).length(), 1);
    }

    #[test]
    fn ordinary_nesting_is_unaffected_by_the_depth_guard() {
        let doc = YrsDoc::new_empty();
        seed_list(&doc, "dst", &[]);
        let src = seed_list(&doc, "src", &[]);
        {
            let mut txn = local_txn(&doc.inner);
            let mut node = src.push_back(&mut txn, MapPrelim::default());
            for _ in 0..8 {
                node = node.insert(&mut txn, "child", MapPrelim::default());
            }
            node.insert(&mut txn, "leaf", "value");
        }

        transfer(
            &doc,
            YrsTransferMode::Copy,
            array_slot(&[key("src")], 0),
            array_slot(&[key("dst")], 0),
        );
        // The whole chain came across, leaf included.
        let mut cursor = any!({});
        let copied = array_state(&doc, "dst");
        let Any::Array(entries) = &copied else {
            panic!("destination is a list, got {copied:?}");
        };
        assert_eq!(entries.len(), 1, "exactly one entry was copied");
        cursor.clone_from(&entries[0]);
        for _ in 0..8 {
            let Any::Map(map) = &cursor else {
                panic!("expected a nested map, got {cursor:?}");
            };
            let next = map.get("child").expect("chain link survives").clone();
            cursor = next;
        }
        let Any::Map(deepest) = &cursor else {
            panic!("expected the deepest map, got {cursor:?}");
        };
        assert_eq!(deepest.get("leaf"), Some(&Any::from("value")));
    }
}
