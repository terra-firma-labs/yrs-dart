//! YUndoManager wrapper.
//!
//! Tracks transactions tagged with [`YRS_DART_LOCAL_ORIGIN`] only — remote
//! updates applied via `YrsDoc::apply_update` (tagged with
//! `YRS_DART_REMOTE_ORIGIN`) deliberately do NOT enter the undo stack.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use flutter_rust_bridge::frb;
use yrs::sync::{Clock, Timestamp};
use yrs::undo::{Options as UndoOptions, StackItem, UndoManager};
use yrs::Origin;

use crate::api::origin::YRS_DART_LOCAL_ORIGIN;
use crate::api::yrs_array::YrsArray;
use crate::api::yrs_doc::YrsDoc;
use crate::api::yrs_map::YrsMap;
use crate::api::yrs_text::YrsText;

/// A `Clock` impl that returns wall-clock millis on every supported target.
/// yrs ships `SystemClock` only for non-wasm; we provide our own so the
/// undo merge-window timer works the same on Web `--wasm` as everywhere else.
struct CrossClock;

impl Clock for CrossClock {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn now(&self) -> Timestamp {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before UNIX epoch — undo merge window cannot function")
            .as_millis() as Timestamp
    }

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    fn now(&self) -> Timestamp {
        js_sys::Date::now() as Timestamp
    }
}

fn build_undo_options(capture_timeout_millis: u64) -> UndoOptions<()> {
    let mut tracked_origins = HashSet::new();
    tracked_origins.insert(Origin::from(YRS_DART_LOCAL_ORIGIN));
    UndoOptions {
        capture_timeout_millis,
        tracked_origins,
        capture_transaction: None,
        timestamp: Arc::new(CrossClock),
        init_undo_stack: Vec::new(),
        init_redo_stack: Vec::new(),
    }
}

/// Container handle scoped by an [`YrsUndoManager`].
///
/// Exists because frb cannot bridge `&dyn AsRef<Branch>` across the FFI; the
/// enum gives Dart a concrete tagged type while the Rust side dispatches to
/// `UndoManager::expand_scope` per-variant.
#[derive(Clone)]
pub enum YrsScopeItem {
    Map(YrsMap),
    Array(YrsArray),
    Text(YrsText),
}

impl YrsScopeItem {
    #[frb(sync)]
    pub fn from_map(map: &YrsMap) -> YrsScopeItem {
        YrsScopeItem::Map(map.clone())
    }

    #[frb(sync)]
    pub fn from_array(array: &YrsArray) -> YrsScopeItem {
        YrsScopeItem::Array(array.clone())
    }

    #[frb(sync)]
    pub fn from_text(text: &YrsText) -> YrsScopeItem {
        YrsScopeItem::Text(text.clone())
    }
}

/// Undo/redo history for one document.
///
/// **Use one manager, and the document it tracks, from a single thread.** The
/// internal lock makes concurrent calls memory-safe, but the underlying
/// `UndoManager` cannot survive them: its `Drop` unwraps an exclusive
/// acquisition of the document to detach its observers, and dropping one while
/// any other thread holds a transaction panics inside a destructor, which
/// aborts the process rather than unwinding. Managers are dropped routinely —
/// every history operation builds and drops temporary ones — so this is not an
/// edge case under real concurrency. The binding's own callers are
/// single-threaded per document, which is why the constraint costs nothing.
#[frb(opaque)]
pub struct YrsUndoManager {
    inner: Mutex<Option<UndoState>>,
}

struct CancelableCapture {
    id: u64,
    /// Pre-session undo history, held outside the active manager for the whole
    /// session so that nothing the session does can reach it.
    ///
    /// A depth check could not have bounded this. `UndoManager::undo` pops
    /// stack items in a loop until one produces a visible change, discarding
    /// every item it passes over; a session whose net effect is nothing — an
    /// insert and its own deletion, say — therefore falls straight through its
    /// own items and consumes older history in the same call, whatever was
    /// checked beforehand. Withholding the history instead makes the floor the
    /// bottom of the manager's own stack, which yrs enforces by returning
    /// `None` when it runs out of items.
    undo_stack: Vec<StackItem<()>>,
    /// Pre-session redo history, held for the same reason: a temporary session
    /// write would otherwise release it before a cancellation could restore it.
    redo_stack: Vec<StackItem<()>>,
    /// Whether cancelling can still walk the document back.
    ///
    /// `clear()` destroys the session's own stack items along with everything
    /// else, leaving nothing to revert. Reverting is then impossible rather
    /// than merely unperformed, so the capture records it instead of silently
    /// reverting part of the session.
    revertible: bool,
}

struct UndoState {
    manager: UndoManager<()>,
    scope: Vec<YrsScopeItem>,
    capture_timeout_millis: u64,
    active_capture: Option<CancelableCapture>,
    next_capture_id: u64,
}

impl UndoState {
    /// Total local history depth, counting history an open capture is holding
    /// outside the active manager.
    fn total_undo_len(&self) -> usize {
        self.active_capture
            .as_ref()
            .map_or(0, |capture| capture.undo_stack.len())
            + self.manager.undo_stack().len()
    }

    /// Total redo depth, counting history an open capture is holding.
    fn total_redo_len(&self) -> usize {
        self.active_capture
            .as_ref()
            .map_or(0, |capture| capture.redo_stack.len())
            + self.manager.redo_stack().len()
    }
}

fn expand(mgr: &mut UndoManager<()>, item: YrsScopeItem) {
    match item {
        YrsScopeItem::Map(m) => mgr.expand_scope(&m.inner),
        YrsScopeItem::Array(a) => mgr.expand_scope(&a.inner),
        YrsScopeItem::Text(t) => mgr.expand_scope(&t.inner),
    }
}

/// Replace the manager while transferring explicit history stacks.
///
/// Dropping the previous manager only detaches its observers; stack-item
/// retention is transferred to the replacement and is released by the
/// replacement's ordinary clear/pop lifecycle.
fn replace_manager(
    state: &mut UndoState,
    capture_timeout_millis: u64,
    undo_stack: Vec<StackItem<()>>,
    redo_stack: Vec<StackItem<()>>,
) {
    let doc = state.manager.doc().clone();
    let mut options = build_undo_options(capture_timeout_millis);
    options.init_undo_stack = undo_stack;
    options.init_redo_stack = redo_stack;
    let mut replacement = UndoManager::with_options(&doc, options);
    for item in state.scope.iter().cloned() {
        expand(&mut replacement, item);
    }
    state.manager = replacement;
}

/// Collapse consecutive stack items into the single item yrs would have built
/// had one merge window covered all of them. Items must be in stack order:
/// merging is `IdSet` union, which is how the merge window itself extends the
/// newest item.
fn merge_stack_items(items: Vec<StackItem<()>>) -> Option<StackItem<()>> {
    let mut items = items.into_iter();
    let mut merged = items.next()?;
    for item in items {
        merged.merge(item, |_: &mut (), _: ()| {});
    }
    Some(merged)
}

/// Release retained structs owned by history that will not be transferred to
/// the next manager.
///
/// Dropping a [`StackItem`] leaves the `keep` flags its deletions set on the
/// document; only `UndoManager::clear` unsets them. Items held outside a
/// manager therefore have to be handed to one to be discarded.
fn release_stack_items(state: &UndoState, items: Vec<StackItem<()>>) {
    if items.is_empty() {
        return;
    }
    let doc = state.manager.doc().clone();
    let mut options = build_undo_options(state.capture_timeout_millis);
    options.init_redo_stack = items;
    let mut cleanup = UndoManager::with_options(&doc, options);
    for item in state.scope.iter().cloned() {
        expand(&mut cleanup, item);
    }
    cleanup.clear();
}

impl YrsUndoManager {
    pub fn new(doc: &YrsDoc, scope: Vec<YrsScopeItem>, capture_timeout_millis: u64) -> Self {
        let options = build_undo_options(capture_timeout_millis);
        let mut mgr = UndoManager::with_options(&doc.inner, options);
        for item in scope.iter().cloned() {
            expand(&mut mgr, item);
        }
        YrsUndoManager {
            inner: Mutex::new(Some(UndoState {
                manager: mgr,
                scope,
                capture_timeout_millis,
                active_capture: None,
                next_capture_id: 1,
            })),
        }
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut UndoManager<()>) -> R) -> Option<R> {
        self.inner
            .lock()
            .unwrap()
            .as_mut()
            .map(|state| f(&mut state.manager))
    }

    fn with_ref<R>(&self, f: impl FnOnce(&UndoManager<()>) -> R) -> Option<R> {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|state| f(&state.manager))
    }

    #[frb(sync)]
    pub fn add_scope(&self, item: YrsScopeItem) {
        if let Some(state) = self.inner.lock().unwrap().as_mut() {
            expand(&mut state.manager, item.clone());
            state.scope.push(item);
        }
    }

    /// Undo one step.
    ///
    /// While a cancelable capture is open the manager holds only what that
    /// session pushed, so this reverts the session's own work and then stops:
    /// pre-session history is held outside the manager and is unreachable until
    /// the capture closes. That is what keeps "cancel reverts exactly this
    /// session" true however the session behaves.
    #[frb(sync)]
    pub fn undo(&self) -> bool {
        self.with_mut(|m| m.undo_blocking()).unwrap_or(false)
    }

    #[frb(sync)]
    pub fn redo(&self) -> bool {
        self.with_mut(|m| m.redo_blocking()).unwrap_or(false)
    }

    /// Whether [`Self::undo`] would do anything now. While a capture is open
    /// this covers the session's own work only, for the reason described there.
    #[frb(sync)]
    pub fn can_undo(&self) -> bool {
        self.with_ref(|m| m.can_undo()).unwrap_or(false)
    }

    /// Whether [`Self::redo`] would do anything now. While a capture is open
    /// this covers the session's own work only.
    #[frb(sync)]
    pub fn can_redo(&self) -> bool {
        self.with_ref(|m| m.can_redo()).unwrap_or(false)
    }

    /// Total number of local undo stack items, including any an open capture is
    /// holding outside the manager. Use [`Self::can_undo`] to ask what is
    /// reachable right now.
    ///
    /// Exposed so callers can key their own per-step state (a cursor or
    /// selection to restore, say) to an exact stack depth, rather than having to
    /// infer it from how many edits happened to merge into one item. A depth is
    /// **not a stable identifier for a step**: finishing a capture collapses
    /// everything the session pushed into one item, so the depth falls, and
    /// later writes re-issue the depths the collapse freed. The same number can
    /// therefore name different history states across a capture boundary.
    #[frb(sync)]
    pub fn undo_stack_len(&self) -> u64 {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map_or(0, |state| state.total_undo_len() as u64)
    }

    /// Total number of local redo stack items, including any an open capture is
    /// holding. See [`Self::undo_stack_len`].
    #[frb(sync)]
    pub fn redo_stack_len(&self) -> u64 {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map_or(0, |state| state.total_redo_len() as u64)
    }

    /// Equivalent of Yjs's `stopCapturing()` — forces the next mutation to
    /// start a fresh stack item rather than merging into the previous one.
    #[frb(sync)]
    pub fn reset(&self) {
        self.with_mut(|m| m.reset());
    }

    /// Drop all local history.
    ///
    /// An open capture holds the pre-session stacks outside the manager, where
    /// `UndoManager::clear` cannot reach them; clearing drops those too, so no
    /// close path can hand back history the caller just destroyed.
    ///
    /// Clearing during a capture also destroys the session's own stack items,
    /// which are the only record of what to walk back. The capture therefore
    /// stops being revertible: it still closes either way, but cancelling after
    /// a clear leaves the document as the session left it rather than reverting
    /// part of it. Close the capture before clearing if the session's writes
    /// were meant to be discarded.
    #[frb(sync)]
    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        let Some(state) = guard.as_mut() else {
            return;
        };
        state.manager.clear();
        let stranded = state.active_capture.as_mut().map(|capture| {
            capture.revertible = false;
            (
                std::mem::take(&mut capture.undo_stack),
                std::mem::take(&mut capture.redo_stack),
            )
        });
        if let Some((undo_stack, redo_stack)) = stranded {
            release_stack_items(state, undo_stack);
            release_stack_items(state, redo_stack);
        }
    }

    /// Starts one explicitly bounded capture that may later be cancelled.
    ///
    /// Only one capture may be active. The returned identifier must be passed
    /// back to `finishCancelableCapture` or `cancelCancelableCapture`, either of
    /// which always closes the capture and frees the slot. Returning `None`
    /// leaves the manager unchanged.
    #[frb(sync)]
    pub fn begin_cancelable_capture(&self) -> Option<u64> {
        let mut guard = self.inner.lock().unwrap();
        let state = guard.as_mut()?;
        if state.active_capture.is_some() {
            return None;
        }

        state.manager.reset();
        let undo_stack = state.manager.undo_stack().to_vec();
        let redo_stack = state.manager.redo_stack().to_vec();
        // A text session can exceed the ordinary debounce/capture window by
        // minutes. Use one effectively-unbounded merge window for the active
        // capture, then restore the configured window at finish/cancel.
        //
        // The session starts against empty stacks: pre-session history is held
        // on the capture, out of the manager's reach, so the session can only
        // ever undo its own work and a cancellation always has the whole of
        // that work — and nothing else — to walk back.
        replace_manager(state, u64::MAX, Vec::new(), Vec::new());
        let id = state.next_capture_id;
        state.next_capture_id = state.next_capture_id.wrapping_add(1).max(1);
        state.active_capture = Some(CancelableCapture {
            id,
            undo_stack,
            redo_stack,
            revertible: true,
        });
        Some(id)
    }

    /// Finishes the matching bounded capture and forces the next mutation to
    /// start a new undo item.
    ///
    /// Whatever the session did — one write, a hundred merged keystrokes, an
    /// undo/redo pair, an explicit `reset()` — everything it pushed above the
    /// depth the capture opened at collapses into the single undoable step a
    /// capture means. Only a mismatched/stale identifier or a disposed manager
    /// fails; a live matching capture always closes.
    #[frb(sync)]
    pub fn finish_cancelable_capture(&self, capture_id: u64) -> bool {
        let mut guard = self.inner.lock().unwrap();
        let Some(state) = guard.as_mut() else {
            return false;
        };
        let Some(capture) = state.active_capture.as_ref() else {
            return false;
        };
        if capture.id != capture_id {
            return false;
        }
        let mut undo_stack = capture.undo_stack.clone();
        let saved_redo_stack = capture.redo_stack.clone();
        let capture_timeout_millis = state.capture_timeout_millis;

        state.manager.reset();
        // The manager holds exactly what the session pushed, so its whole undo
        // stack is the session and there is no boundary to compute.
        let session_step = merge_stack_items(state.manager.undo_stack().to_vec());
        let session_redo_stack = state.manager.redo_stack().to_vec();

        let redo_stack = match session_step {
            Some(step) => {
                undo_stack.push(step);
                // A surviving edit invalidates pre-session redo exactly like an
                // ordinary local transaction. It was held outside the active
                // manager, so release it explicitly now.
                release_stack_items(state, saved_redo_stack);
                session_redo_stack
            }
            None => {
                // Nothing of the session survives, so pre-session redo is still
                // valid; anything the session pushed is newer than it.
                let mut redo_stack = saved_redo_stack;
                redo_stack.extend(session_redo_stack);
                redo_stack
            }
        };
        replace_manager(state, capture_timeout_millis, undo_stack, redo_stack);
        state.active_capture = None;
        true
    }

    /// Reverts the whole matching bounded capture and permanently discards the
    /// redo history that reverting it produced.
    ///
    /// The undo depth is walked back to exactly where the capture opened,
    /// however many stack items the session ended up spanning, so the document
    /// returns to its pre-session state. Undo entries older than the capture
    /// remain available, and redo entries that existed before it began are
    /// restored. Only a mismatched/stale identifier or a disposed manager
    /// fails; a live matching capture always closes.
    #[frb(sync)]
    pub fn cancel_cancelable_capture(&self, capture_id: u64) -> bool {
        let mut guard = self.inner.lock().unwrap();
        let Some(state) = guard.as_mut() else {
            return false;
        };
        let Some(capture) = state.active_capture.as_ref() else {
            return false;
        };
        if capture.id != capture_id {
            return false;
        }
        let saved_undo_stack = capture.undo_stack.clone();
        let saved_redo_stack = capture.redo_stack.clone();
        let revertible = capture.revertible;
        let capture_timeout_millis = state.capture_timeout_millis;

        state.manager.reset();
        if revertible {
            // Drain the session. The manager holds only what the session
            // pushed, so exhausting its undo stack *is* the pre-session
            // document — there is no depth to count, and nothing older it could
            // reach even if a step reverts to no visible change and yrs keeps
            // popping. Each successful call consumes at least one item, so the
            // stack length bounds the iterations.
            for _ in 0..state.manager.undo_stack().len() {
                if !state.manager.undo_blocking() {
                    break;
                }
            }
        }

        // Everything the active manager still holds was produced inside the
        // session or by reverting it, so none of it outlives the cancellation.
        // `UndoManager` has no public pop-and-release, so these go through the
        // same temporary-manager clear an ordinary stack drop would use.
        let discarded_redo_stack = state.manager.redo_stack().to_vec();
        release_stack_items(state, discarded_redo_stack);

        let mut undo_stack = saved_undo_stack;
        let session_undo_stack = state.manager.undo_stack().to_vec();
        if revertible {
            release_stack_items(state, session_undo_stack);
        } else {
            // A cleared capture cannot be reverted, so its writes stay in the
            // document. Keep the step that undoes them rather than releasing
            // it: dropping it would leave the user with content they asked to
            // discard and no way back at all — strictly worse than the close
            // path that means "keep this". Collapsed like a finish, so the
            // stranded session is still one undoable step.
            undo_stack.extend(merge_stack_items(session_undo_stack));
        }

        replace_manager(state, capture_timeout_millis, undo_stack, saved_redo_stack);
        state.active_capture = None;
        true
    }

    /// Release the manager and any history a capture is holding.
    ///
    /// Idempotent, and also run from `Drop`, so a manager that goes out of
    /// scope without an explicit call still releases the document structs its
    /// history was retaining.
    #[frb(sync)]
    pub fn dispose(&self) {
        let mut guard = self.inner.lock().unwrap();
        if let Some(state) = guard.as_mut() {
            // Pre-session history is held outside the manager, so disposing
            // while a capture is open would drop it without releasing its
            // structs.
            if let Some(capture) = state.active_capture.take() {
                release_stack_items(state, capture.undo_stack);
                release_stack_items(state, capture.redo_stack);
            }
            // Dropping a manager only detaches its observers; the `keep` flags
            // its own history set on the document are unset by `clear` alone.
            state.manager.clear();
        }
        *guard = None;
    }
}

impl Drop for YrsUndoManager {
    fn drop(&mut self) {
        // A manager that goes out of scope without an explicit dispose would
        // otherwise leave every struct its history retains alive for the
        // document's lifetime.
        //
        // Deliberately not `self.dispose()`: that unwraps the lock, and a
        // panic raised while unwinding a destructor aborts the process. A
        // poisoned lock only means an earlier call panicked, which is not a
        // reason to take the process down, so recover the guard instead.
        let mut guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let Some(state) = guard.as_mut() else {
            return;
        };
        if let Some(capture) = state.active_capture.take() {
            release_stack_items(state, capture.undo_stack);
            release_stack_items(state, capture.redo_stack);
        }
        state.manager.clear();
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::api::origin::local_txn;
    use crate::api::values::YInValue;

    /// A doc with one scoped root array plus one unscoped root array.
    ///
    /// `capture_timeout_millis` is 0 in every test so each ordinary write is
    /// its own stack item and depth assertions are exact.
    fn fixture() -> (YrsDoc, YrsArray, YrsArray, YrsUndoManager) {
        let doc = YrsDoc::new_empty();
        let scoped = doc.get_array("scoped".to_owned());
        let unscoped = doc.get_array("unscoped".to_owned());
        let manager = YrsUndoManager::new(&doc, vec![YrsScopeItem::from_array(&scoped)], 0);
        (doc, scoped, unscoped, manager)
    }

    fn push(list: &YrsArray, value: &str) {
        list.push(YInValue::String(value.to_owned()));
    }

    #[test]
    fn begin_then_finish_collapses_the_session_into_one_undoable_step() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        assert_eq!(manager.undo_stack_len(), 1);

        let id = manager
            .begin_cancelable_capture()
            .expect("a fresh manager must open a capture");
        push(&scoped, "b");
        push(&scoped, "c");
        // The session uses an effectively unbounded merge window, so both
        // writes land on a single stack item.
        assert_eq!(manager.undo_stack_len(), 2);

        assert!(manager.finish_cancelable_capture(id));
        assert_eq!(manager.undo_stack_len(), 2);
        assert_eq!(scoped.json(), r#"["a","b","c"]"#);

        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(manager.redo_stack_len(), 1);
    }

    #[test]
    fn finish_restores_the_configured_merge_window() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "a");
        assert!(manager.finish_cancelable_capture(id));

        // Back on the 0ms window, each write is its own step again.
        push(&scoped, "b");
        push(&scoped, "c");
        assert_eq!(manager.undo_stack_len(), 3);
    }

    #[test]
    fn finish_without_a_write_leaves_the_stack_depth_unchanged() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        assert!(manager.finish_cancelable_capture(id));

        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(scoped.json(), r#"["a"]"#);
    }

    #[test]
    fn cancel_reverts_the_session_write_and_leaves_no_step() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "b");
        assert_eq!(scoped.json(), r#"["a","b"]"#);

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(manager.redo_stack_len(), 0);
        assert!(!manager.can_redo());
    }

    #[test]
    fn cancel_keeps_history_captured_before_the_capture_began() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert_eq!(manager.undo_stack_len(), 2);

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "c");
        assert!(manager.cancel_cancelable_capture(id));

        assert_eq!(manager.undo_stack_len(), 2);
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert!(manager.undo());
        assert_eq!(scoped.json(), "[]");
        assert!(!manager.can_undo());
    }

    #[test]
    fn cancel_restores_redo_history_that_existed_before_the_capture() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert_eq!(manager.redo_stack_len(), 1);

        let id = manager.begin_cancelable_capture().expect("capture opens");
        // Pre-session redo is held outside the active manager for the whole
        // session so a temporary write cannot release it. It still counts
        // toward the reported depth — held, not destroyed — but is not
        // reachable while the capture is open.
        assert_eq!(manager.redo_stack_len(), 1);
        assert!(!manager.can_redo(), "held history is not reachable");
        push(&scoped, "c");
        assert_eq!(manager.redo_stack_len(), 1);
        assert!(!manager.can_redo());

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(manager.redo_stack_len(), 1);
        assert!(manager.redo());
        assert_eq!(scoped.json(), r#"["a","b"]"#);
    }

    #[test]
    fn finish_without_a_write_restores_redo_history_that_existed_before() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.undo());
        assert_eq!(manager.redo_stack_len(), 1);

        let id = manager.begin_cancelable_capture().expect("capture opens");
        assert!(manager.finish_cancelable_capture(id));

        assert_eq!(manager.redo_stack_len(), 1);
        assert!(manager.redo());
        assert_eq!(scoped.json(), r#"["a","b"]"#);
    }

    #[test]
    fn finish_with_a_write_discards_redo_history_that_existed_before() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert_eq!(manager.redo_stack_len(), 1);

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "c");
        assert!(manager.finish_cancelable_capture(id));

        // A committed edit invalidates pre-session redo exactly like an
        // ordinary local transaction would.
        assert_eq!(manager.redo_stack_len(), 0);
        assert!(!manager.can_redo());
        assert_eq!(scoped.json(), r#"["a","c"]"#);

        // Releasing the discarded redo history must not damage live history.
        assert_eq!(manager.undo_stack_len(), 2);
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert!(manager.undo());
        assert_eq!(scoped.json(), "[]");
    }

    #[test]
    fn only_one_capture_may_be_active_at_a_time() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        let first = manager.begin_cancelable_capture().expect("capture opens");

        assert_eq!(manager.begin_cancelable_capture(), None);

        push(&scoped, "a");
        assert!(manager.finish_cancelable_capture(first));
        // The slot is free again once the capture closes.
        let second = manager.begin_cancelable_capture().expect("capture reopens");
        assert_ne!(second, first);
        assert!(manager.finish_cancelable_capture(second));
    }

    #[test]
    fn mismatched_and_stale_capture_ids_fail_closed() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "a");

        assert!(!manager.finish_cancelable_capture(id.wrapping_add(1)));
        assert!(!manager.cancel_cancelable_capture(id.wrapping_add(7)));
        // Neither rejection disturbed the still-active capture.
        assert!(manager.finish_cancelable_capture(id));
        // The now-stale identifier no longer matches anything.
        assert!(!manager.finish_cancelable_capture(id));
        assert!(!manager.cancel_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["a"]"#);
    }

    #[test]
    fn cancel_without_a_session_write_closes_the_capture_and_changes_nothing() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        // Nothing to revert, but a live matching capture always closes — the
        // caller must never be left holding an un-closable capture.
        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(scoped.json(), r#"["a"]"#);

        // The slot is free, and the stale id no longer matches anything.
        assert!(!manager.finish_cancelable_capture(id));
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn a_capture_split_into_several_stack_items_still_finishes_as_one_step() {
        // `reset()` inside a session splits it across stack items. A capture
        // means "one undoable step", so finishing collapses however many items
        // the session produced back into one.
        let (_doc, scoped, _unscoped, manager) = fixture();
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "a");
        manager.reset();
        push(&scoped, "b");
        assert_eq!(manager.undo_stack_len(), 2);

        assert!(manager.finish_cancelable_capture(id));
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(scoped.json(), r#"["a","b"]"#);

        // The slot is free again, and the single step reverts the whole session.
        let next = manager.begin_cancelable_capture().expect("capture reopens");
        assert!(manager.finish_cancelable_capture(next));
        assert!(manager.undo());
        assert_eq!(scoped.json(), "[]");
    }

    #[test]
    fn a_capture_split_into_several_stack_items_still_cancels_whole() {
        // The cancel counterpart: a multi-item session reverts entirely, back
        // to exactly the depth the capture opened at.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "base");
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "a");
        manager.reset();
        push(&scoped, "b");
        assert_eq!(manager.undo_stack_len(), 3);

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["base"]"#);
        assert_eq!(manager.undo_stack_len(), 1);
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn an_undo_redo_pair_inside_a_capture_leaves_it_recoverable() {
        // The exact production sequence that used to produce the dead state:
        // type, undo, redo, type again. yrs zeroes `last_change` during
        // undo/redo, so the second write starts a fresh stack item.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "base");
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "t1");
        assert!(manager.undo());
        assert!(manager.redo());
        push(&scoped, "t2");

        assert!(manager.finish_cancelable_capture(id));
        // The capture slot is usable again and the merge window was restored:
        // with a zero capture timeout each later write is its own step.
        let before = manager.undo_stack_len();
        push(&scoped, "after1");
        push(&scoped, "after2");
        assert_eq!(manager.undo_stack_len(), before + 2);
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn writes_outside_the_scope_do_not_count_as_session_writes() {
        let (_doc, scoped, unscoped, manager) = fixture();
        push(&scoped, "a");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&unscoped, "ignored");
        assert_eq!(manager.undo_stack_len(), 1);

        assert!(manager.finish_cancelable_capture(id));
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(unscoped.json(), r#"["ignored"]"#);

        assert!(manager.undo());
        assert_eq!(scoped.json(), "[]");
        assert_eq!(unscoped.json(), r#"["ignored"]"#);
    }

    #[test]
    fn a_disposed_manager_rejects_every_capture_operation() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        let id = manager.begin_cancelable_capture().expect("capture opens");
        manager.dispose();

        assert_eq!(manager.begin_cancelable_capture(), None);
        assert!(!manager.finish_cancelable_capture(id));
        assert!(!manager.cancel_cancelable_capture(id));
        assert_eq!(manager.undo_stack_len(), 0);
        assert!(!manager.undo());
    }

    #[test]
    fn replace_manager_transfers_undo_and_redo_depth_and_scope() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.undo());
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(manager.redo_stack_len(), 1);

        {
            let mut guard = manager.inner.lock().unwrap();
            let state = guard.as_mut().expect("state must be live");
            let undo_stack = state.manager.undo_stack().to_vec();
            let redo_stack = state.manager.redo_stack().to_vec();
            replace_manager(state, 0, undo_stack, redo_stack);
        }

        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(manager.redo_stack_len(), 1);
        // The transferred stacks still drive the document.
        assert!(manager.redo());
        assert_eq!(scoped.json(), r#"["a","b"]"#);
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);

        // The scope survived the swap, so new writes are still tracked.
        push(&scoped, "c");
        assert_eq!(manager.undo_stack_len(), 2);
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
    }

    #[test]
    fn release_stack_items_ignores_an_empty_stack() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        assert!(manager.undo());

        {
            let guard = manager.inner.lock().unwrap();
            let state = guard.as_ref().expect("state must be live");
            release_stack_items(state, Vec::new());
        }

        assert_eq!(manager.redo_stack_len(), 1);
        assert!(manager.redo());
        assert_eq!(scoped.json(), r#"["a"]"#);
    }

    #[test]
    fn capture_identifiers_are_not_reused_across_sessions() {
        let (_doc, _scoped, _unscoped, manager) = fixture();
        let mut seen = Vec::new();
        for _ in 0..3 {
            let id = manager.begin_cancelable_capture().expect("capture opens");
            assert!(manager.finish_cancelable_capture(id));
            seen.push(id);
        }

        assert_eq!(seen, vec![1, 2, 3]);
    }

    #[test]
    fn a_cancelled_session_can_be_followed_by_a_fresh_one() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");

        let first = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "b");
        assert!(manager.cancel_cancelable_capture(first));

        let second = manager.begin_cancelable_capture().expect("capture reopens");
        push(&scoped, "c");
        assert!(manager.finish_cancelable_capture(second));

        assert_eq!(scoped.json(), r#"["a","c"]"#);
        assert_eq!(manager.undo_stack_len(), 2);
    }

    #[test]
    fn cancel_reverts_a_session_that_undid_and_redid_inside_itself() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "b");
        assert!(manager.undo());
        assert!(manager.redo());
        push(&scoped, "c");

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert_eq!(manager.undo_stack_len(), 1);
        assert_eq!(manager.redo_stack_len(), 0);
    }

    #[test]
    fn cancel_after_a_session_undid_its_own_writes_restores_pre_session_state() {
        // A session may undo its own writes; it simply runs out of items to
        // undo, so cancelling always lands on exactly the pre-session document.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "c");
        assert!(manager.undo());
        assert!(!manager.undo(), "nothing of the session is left to undo");
        assert_eq!(scoped.json(), r#"["a","b"]"#);
        assert_eq!(manager.undo_stack_len(), 2);

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["a","b"]"#);
        assert_eq!(manager.undo_stack_len(), 2);
        // The session's own redo entry does not outlive the cancellation.
        assert!(!manager.can_redo());
    }

    #[test]
    fn a_fully_undone_session_still_finishes() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "c");
        assert!(manager.undo(), "the session's own write is undoable");
        assert!(!manager.undo(), "and \"b\" is not the session's to consume");
        assert_eq!(scoped.json(), r#"["a","b"]"#);

        // The session undid everything it wrote, so it leaves no step to
        // collapse. Finishing is still the caller's way out.
        assert!(manager.finish_cancelable_capture(id));
        assert_eq!(manager.undo_stack_len(), 2);
        assert_eq!(scoped.json(), r#"["a","b"]"#);
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn cancel_restores_the_configured_merge_window() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "a");
        assert!(manager.cancel_cancelable_capture(id));

        // Back on the 0ms window, each write is its own step again.
        push(&scoped, "b");
        push(&scoped, "c");
        assert_eq!(manager.undo_stack_len(), 2);
    }

    /// One thing a caller might do between opening and closing a capture.
    type SessionShape = fn(&YrsArray, &YrsUndoManager);

    #[test]
    fn no_session_shape_can_strand_the_capture_slot() {
        // The invariant behind every case above, swept over the session shapes
        // that used to break one close path or the other: whichever close the
        // caller picks succeeds, the slot comes back, and the configured merge
        // window is in force again afterwards.
        let shapes: Vec<(&str, SessionShape)> = vec![
            ("nothing", |_, _| {}),
            ("one write", |scoped, _| push(scoped, "w")),
            ("many writes", |scoped, _| {
                push(scoped, "w");
                push(scoped, "x");
                push(scoped, "y");
            }),
            ("reset-split writes", |scoped, manager| {
                push(scoped, "w");
                manager.reset();
                push(scoped, "x");
            }),
            ("undo/redo pair", |scoped, manager| {
                push(scoped, "w");
                manager.undo();
                manager.redo();
                push(scoped, "x");
            }),
            ("fully undone session", |scoped, manager| {
                push(scoped, "w");
                manager.undo();
            }),
            ("undo past the start", |scoped, manager| {
                push(scoped, "w");
                manager.undo();
                manager.undo();
                manager.undo();
            }),
            ("reset only", |_, manager| manager.reset()),
            ("clear", |scoped, manager| {
                push(scoped, "w");
                manager.clear();
            }),
        ];

        for (shape, run) in shapes {
            for closes_with_finish in [true, false] {
                let (_doc, scoped, _unscoped, manager) = fixture();
                push(&scoped, "seed");
                let id = manager
                    .begin_cancelable_capture()
                    .unwrap_or_else(|| panic!("{shape}: capture must open"));
                run(&scoped, &manager);

                let closed = if closes_with_finish {
                    manager.finish_cancelable_capture(id)
                } else {
                    manager.cancel_cancelable_capture(id)
                };
                assert!(closed, "{shape}: close must succeed ({closes_with_finish})");

                let next = manager
                    .begin_cancelable_capture()
                    .unwrap_or_else(|| panic!("{shape}: slot must be free ({closes_with_finish})"));
                assert!(manager.finish_cancelable_capture(next));

                let before = manager.undo_stack_len();
                push(&scoped, "after");
                push(&scoped, "after-again");
                assert_eq!(
                    manager.undo_stack_len(),
                    before + 2,
                    "{shape}: merge window must be restored ({closes_with_finish})"
                );
            }
        }
    }

    #[test]
    fn cancel_restores_pre_session_state_when_the_session_undid_then_wrote() {
        // Withholding pre-session history exists for exactly this shape: undo
        // inside an open session, then keep writing. When the session could
        // reach older history, that undo consumed it, the following write
        // cleared its redo entry, and cancel had no way back — the session
        // write survived and the older step was destroyed.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert_eq!(scoped.json(), r#"["a","b"]"#);

        let id = manager.begin_cancelable_capture().expect("capture opens");
        // Nothing to undo yet: the session has pushed nothing of its own.
        assert!(!manager.can_undo());
        assert!(!manager.undo(), "must not undo past the capture start");
        push(&scoped, "c");
        // Now the session has an item of its own, so undo reaches it.
        assert!(manager.can_undo());

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(
            scoped.json(),
            r#"["a","b"]"#,
            "cancel returns the document to its pre-session state"
        );
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn a_capture_cannot_consume_history_older_than_itself() {
        // The worse form of the same shape: repeated undos used to walk the
        // document out from under the session entirely.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        for _ in 0..5 {
            assert!(!manager.undo(), "an empty session has nothing to undo");
        }
        push(&scoped, "c");

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["a","b"]"#);
    }

    #[test]
    fn withholding_only_applies_while_a_capture_is_open() {
        // Ordinary undo is untouched outside a session, and is restored the
        // moment the capture closes.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.can_undo());

        let id = manager.begin_cancelable_capture().expect("capture opens");
        assert!(!manager.can_undo(), "older history is withheld while open");
        assert!(manager.finish_cancelable_capture(id));

        assert!(
            manager.can_undo(),
            "history returns when the capture closes"
        );
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
    }

    #[test]
    fn a_session_may_still_undo_its_own_writes() {
        // Withholding bounds the session, it does not freeze it.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "base");
        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "s1");
        manager.reset();
        push(&scoped, "s2");

        assert!(manager.undo(), "the session's own writes remain undoable");
        assert!(manager.undo());
        assert!(!manager.undo(), "but only its own work");
        assert_eq!(scoped.json(), r#"["base"]"#);

        assert!(manager.finish_cancelable_capture(id));
        assert_eq!(scoped.json(), r#"["base"]"#);
    }

    #[test]
    fn a_dropped_manager_releases_the_structs_its_history_retained() {
        // yrs keeps a deleted struct alive while a stack item could still
        // restore it, so history pins document memory until it is cleared.
        // Only `UndoManager::clear` unsets those flags, and dropping a manager
        // does not call it — so a manager that went out of scope without an
        // explicit dispose pinned everything it had ever tracked for the
        // document's lifetime.
        fn retained_size(build_manager: bool) -> usize {
            let doc = YrsDoc::new_empty();
            let scoped = doc.get_array("scoped".to_owned());
            {
                let manager = build_manager
                    .then(|| YrsUndoManager::new(&doc, vec![YrsScopeItem::from_array(&scoped)], 0));
                push(&scoped, "x".repeat(400).as_str());
                scoped.remove_at(0);
                // Dropped here, without dispose, if one was built at all.
                drop(manager);
            }
            // yrs collects only the committing transaction's delete set, so the
            // release is invisible without forcing a pass.
            {
                let mut txn = local_txn(&doc.inner);
                txn.gc(None);
            }
            doc.save().len()
        }

        let baseline = retained_size(false);
        let with_manager = retained_size(true);
        assert_eq!(
            with_manager, baseline,
            "a dropped manager must not pin the structs its history tracked"
        );
    }

    #[test]
    fn a_net_neutral_session_cannot_consume_older_history() {
        // The shape that defeated every depth check. `UndoManager::undo` pops
        // stack items until one produces a visible change and discards the
        // rest, so a session that inserts and then deletes its own entry is
        // invisible: one undo fell through it and consumed the pre-session
        // step behind it, the next write cleared that step's redo entry, and
        // cancellation had nothing left to restore. Nothing about the depth at
        // the moment of the call revealed this was about to happen.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert_eq!(scoped.json(), r#"["a","b"]"#);

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "c");
        scoped.remove_at(2);
        assert_eq!(scoped.json(), r#"["a","b"]"#, "the session is net-neutral");

        // The session may undo its own invisible work; it may not reach past
        // it, because pre-session history is not in the manager at all.
        while manager.undo() {}
        assert_eq!(
            scoped.json(),
            r#"["a","b"]"#,
            "no amount of undoing reaches \"b\""
        );

        push(&scoped, "d");
        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(
            scoped.json(),
            r#"["a","b"]"#,
            "cancel lands on the pre-session document"
        );

        // And the pre-session history it was protecting is still usable.
        assert!(manager.undo());
        assert_eq!(scoped.json(), r#"["a"]"#);
        assert!(manager.undo());
        assert_eq!(scoped.json(), "[]");
    }

    #[test]
    fn cancel_restores_the_pre_session_document_whatever_the_session_did() {
        // The invariant itself rather than any one route to breaking it: for
        // every session shape, cancelling puts the document back exactly as it
        // was and leaves the history that existed before it intact. Asserted
        // against the recorded document and depths, so a shape that reverts
        // "to the right stack depth" but the wrong content still fails.
        let shapes: Vec<(&str, SessionShape)> = vec![
            ("nothing", |_, _| {}),
            ("one write", |scoped, _| push(scoped, "w")),
            ("many writes", |scoped, _| {
                push(scoped, "w");
                push(scoped, "x");
                push(scoped, "y");
            }),
            ("net-neutral write", |scoped, _| {
                push(scoped, "w");
                scoped.remove_at(scoped.length() - 1);
            }),
            ("net-neutral then write", |scoped, _| {
                push(scoped, "w");
                scoped.remove_at(scoped.length() - 1);
                push(scoped, "x");
            }),
            ("net-neutral, undo, write", |scoped, manager| {
                push(scoped, "w");
                scoped.remove_at(scoped.length() - 1);
                manager.undo();
                push(scoped, "x");
            }),
            ("undo storm then write", |scoped, manager| {
                push(scoped, "w");
                for _ in 0..5 {
                    manager.undo();
                }
                push(scoped, "x");
            }),
            ("reset-split writes", |scoped, manager| {
                push(scoped, "w");
                manager.reset();
                push(scoped, "x");
            }),
            ("undo/redo pair", |scoped, manager| {
                push(scoped, "w");
                manager.undo();
                manager.redo();
                push(scoped, "x");
            }),
            ("deletes a pre-session entry", |scoped, _| {
                scoped.remove_at(0);
            }),
            ("deletes then re-adds", |scoped, _| {
                scoped.remove_at(0);
                push(scoped, "seed");
            }),
        ];

        for (shape, run) in shapes {
            let (_doc, scoped, _unscoped, manager) = fixture();
            push(&scoped, "seed");
            push(&scoped, "second");
            // Leave a redo entry outstanding so the shape is exercised against
            // history in both directions.
            assert!(manager.undo());
            let document_before = scoped.json();
            let undo_before = manager.undo_stack_len();
            let redo_before = manager.redo_stack_len();

            let id = manager
                .begin_cancelable_capture()
                .unwrap_or_else(|| panic!("{shape}: capture must open"));
            run(&scoped, &manager);
            assert!(
                manager.cancel_cancelable_capture(id),
                "{shape}: cancel must close"
            );

            assert_eq!(scoped.json(), document_before, "{shape}: document");
            assert_eq!(manager.undo_stack_len(), undo_before, "{shape}: undo depth");
            assert_eq!(manager.redo_stack_len(), redo_before, "{shape}: redo depth");
            // The restored history still drives the document.
            assert!(manager.redo(), "{shape}: pre-session redo is usable");
            assert_eq!(scoped.json(), r#"["seed","second"]"#, "{shape}: after redo");
        }
    }

    #[test]
    fn finish_keeps_pre_session_history_whatever_the_session_did() {
        // The finish counterpart: whatever the session did, history older than
        // the capture is still there afterwards and still reverts correctly.
        let shapes: Vec<(&str, SessionShape)> = vec![
            ("one write", |scoped, _| push(scoped, "w")),
            ("net-neutral write", |scoped, _| {
                push(scoped, "w");
                scoped.remove_at(scoped.length() - 1);
            }),
            ("undo storm then write", |scoped, manager| {
                push(scoped, "w");
                for _ in 0..5 {
                    manager.undo();
                }
                push(scoped, "x");
            }),
            ("fully undone session", |scoped, manager| {
                push(scoped, "w");
                manager.undo();
            }),
        ];

        for (shape, run) in shapes {
            let (_doc, scoped, _unscoped, manager) = fixture();
            push(&scoped, "a");
            push(&scoped, "b");

            let id = manager
                .begin_cancelable_capture()
                .unwrap_or_else(|| panic!("{shape}: capture must open"));
            run(&scoped, &manager);
            assert!(
                manager.finish_cancelable_capture(id),
                "{shape}: finish must close"
            );

            // However many steps the session left, the two pre-session ones are
            // still under them and still revert in order.
            while manager.undo() {}
            assert_eq!(scoped.json(), "[]", "{shape}: pre-session history intact");
        }
    }

    #[test]
    fn clear_during_a_capture_makes_the_capture_unrevertible() {
        // `clear()` destroys the session's own stack items along with
        // everything else, so there is nothing left to walk back. The capture
        // still closes — a caller must never be stranded — but the document
        // keeps what the session wrote rather than being partly reverted.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "before_clear");
        manager.clear();
        assert_eq!(manager.undo_stack_len(), 0, "clear means clear");
        assert_eq!(manager.redo_stack_len(), 0);
        push(&scoped, "after_clear");

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(
            scoped.json(),
            r#"["a","before_clear","after_clear"]"#,
            "a cleared capture cannot revert, and must not revert partly"
        );
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn cancelling_a_cleared_capture_is_never_worse_than_finishing_one() {
        // Cancelling means "discard this". If a `clear()` made that impossible,
        // the writes stay — but the user must not be left holding content they
        // asked to discard with no way back, which would make the discard
        // gesture strictly worse than the keep gesture. Both close paths leave
        // the same document and the same single undoable step.
        let mut outcomes = Vec::new();
        for closes_with_finish in [true, false] {
            let (_doc, scoped, _unscoped, manager) = fixture();
            push(&scoped, "a");

            let id = manager.begin_cancelable_capture().expect("capture opens");
            push(&scoped, "before_clear");
            manager.clear();
            push(&scoped, "after_clear");

            assert!(if closes_with_finish {
                manager.finish_cancelable_capture(id)
            } else {
                manager.cancel_cancelable_capture(id)
            });

            let document = scoped.json();
            let depth = manager.undo_stack_len();
            // Whatever survived is undoable, so the writes are recoverable.
            assert!(manager.undo(), "the stranded session must be undoable");
            outcomes.push((document, depth, scoped.json()));
        }

        assert_eq!(outcomes[0], outcomes[1], "finish and cancel must agree");
        assert_eq!(outcomes[0].0, r#"["a","before_clear","after_clear"]"#);
        assert_eq!(outcomes[0].1, 1, "collapsed into one step");
    }

    #[test]
    fn clear_during_a_capture_does_not_resurrect_destroyed_redo() {
        // The pre-session redo stack lives outside the manager, so a plain
        // `UndoManager::clear` could not reach it and both close paths used to
        // hand it back — returning history the caller had explicitly dropped.
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.undo());
        assert!(manager.can_redo(), "\"b\" is redoable before the capture");

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "session");
        manager.clear();
        assert_eq!(manager.undo_stack_len(), 0);
        assert_eq!(manager.redo_stack_len(), 0);

        assert!(manager.cancel_cancelable_capture(id));
        assert_eq!(manager.redo_stack_len(), 0, "cleared history stays cleared");
        assert!(!manager.can_redo());
        assert!(manager.begin_cancelable_capture().is_some());
    }

    #[test]
    fn clear_during_a_capture_leaves_finish_consistent_too() {
        let (_doc, scoped, _unscoped, manager) = fixture();
        push(&scoped, "a");
        push(&scoped, "b");
        assert!(manager.undo());

        let id = manager.begin_cancelable_capture().expect("capture opens");
        push(&scoped, "session");
        manager.clear();

        assert!(manager.finish_cancelable_capture(id));
        assert_eq!(manager.redo_stack_len(), 0);
        assert!(!manager.can_redo());
        assert!(manager.begin_cancelable_capture().is_some());
    }
}
