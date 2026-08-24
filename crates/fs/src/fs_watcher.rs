use anyhow::Result;
use gpui::BackgroundExecutor;
use notify::{EventKind, EventKindMask, Watcher as _};
use parking_lot::Mutex;
use smol::{Timer, channel::Sender};
use std::{
    collections::{BTreeMap, HashMap},
    ops::DerefMut,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::Duration,
};
use util::{ResultExt, paths::SanitizedPath};

use crate::{PathEvent, PathEventKind, Watcher};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WatcherMode {
    #[default]
    Native,
    Poll {
        interval_ms: u32,
    },
}

pub struct FsWatcher {
    tx: Sender<()>,
    pending_path_events: Arc<Mutex<Vec<PathEvent>>>,
    registrations: Mutex<BTreeMap<Arc<Path>, WatcherRegistrationId>>,
    poll_fallback: Mutex<Option<Box<PollFsWatcher>>>,
    poll_interval: Duration,
}

impl FsWatcher {
    pub fn new(tx: Sender<()>, pending_path_events: Arc<Mutex<Vec<PathEvent>>>, poll_interval: Duration) -> Self {
        Self {
            tx,
            pending_path_events,
            registrations: Default::default(),
            poll_fallback: Default::default(),
            poll_interval,
        }
    }

    fn poll_path(&self, path: &Path) -> Result<()> {
        let mut fallback = self.poll_fallback.lock();
        if fallback.is_none() {
            *fallback = Some(Box::new(PollFsWatcher::new(
                self.tx.clone(),
                self.pending_path_events.clone(),
                self.poll_interval,
            )?));
        }
        fallback.as_ref().unwrap().add(&path)
    }
}

pub struct PendingWatcher(Arc<PendingWatcherImpl>);

struct PendingWatcherImpl {
    watcher: Arc<dyn Watcher>,
    executor: BackgroundExecutor,
    tx: Sender<()>,
    pending_path_events: Arc<Mutex<Vec<PathEvent>>>,
    poll_interval: Duration,
    pending_paths: Mutex<HashMap<PathBuf, Sender<()>>>,
}

impl PendingWatcher {
    pub fn new(
        watcher: Arc<dyn Watcher>,
        executor: BackgroundExecutor,
        tx: Sender<()>,
        pending_path_events: Arc<Mutex<Vec<PathEvent>>>,
        poll_interval: Duration,
    ) -> Self {
        Self(Arc::new(PendingWatcherImpl {
            watcher,
            executor,
            tx,
            pending_path_events,
            poll_interval,
            pending_paths: Mutex::default(),
        }))
    }
}

impl PendingWatcherImpl {
    fn add_pending_path(self: &Arc<Self>, path: &Path) {
        let path = path.to_path_buf();
        let mut pending_paths = self.pending_paths.lock();
        if pending_paths.contains_key(&path) {
            return;
        }

        let (tx, rx) = smol::channel::bounded(1);
        pending_paths.insert(path.clone(), tx);
        drop(pending_paths);

        let this = Arc::downgrade(self);
        let poll_interval = self.poll_interval;
        self.executor
            .spawn(async move {
                loop {
                    if rx.try_recv().is_ok() {
                        break;
                    }

                    match path.try_exists() {
                        Ok(true) => {
                            if let Some(this) = this.upgrade() {
                                this.handle_path_created(&path);
                            }
                            break;
                        }
                        Ok(false) => {}
                        Err(e) => log::warn!("Failed to check pending watch {path:?}: {e}"),
                    }

                    Timer::after(poll_interval).await;
                }
            })
            .detach();
    }

    fn cancel_pending_path(&self, path: &Path) -> bool {
        if let Some(tx) = self.pending_paths.lock().remove(path) {
            tx.try_send(()).is_ok()
        } else {
            false
        }
    }

    fn handle_path_created(&self, path: &PathBuf) {
        match self.watcher.add(path) {
            Ok(()) => {
                self.pending_paths.lock().remove(path);
                push_path_events(
                    &self.tx,
                    &self.pending_path_events,
                    vec![PathEvent {
                        path: path.clone(),
                        kind: Some(PathEventKind::Created),
                    }],
                );
            }
            Err(e) => log::warn!("Failed to add watch for {path:?}: {e}"),
        }
    }
}

impl Drop for PendingWatcherImpl {
    fn drop(&mut self) {
        for (_, tx) in self.pending_paths.get_mut().drain() {
            tx.try_send(()).ok();
        }
    }
}

impl Watcher for PendingWatcher {
    fn add(&self, path: &Path) -> Result<()> {
        match self.0.watcher.add(path) {
            Ok(()) => Ok(()),
            Err(e) => {
                if !path.exists() {
                    log::trace!("Pending watch add for {path:?}: {e}");
                    self.0.add_pending_path(path);
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn remove(&self, path: &Path) -> Result<()> {
        if self.0.cancel_pending_path(path) {
            Ok(())
        } else {
            self.0.watcher.remove(path)
        }
    }
}

impl Drop for FsWatcher {
    fn drop(&mut self) {
        let mut registrations = BTreeMap::new();
        {
            let old = &mut self.registrations.lock();
            std::mem::swap(old.deref_mut(), &mut registrations);
        }

        let _ = global(|g| {
            for (_, registration) in registrations {
                g.remove(registration);
            }
        });
    }
}

impl Watcher for FsWatcher {
    fn add(&self, path: &Path) -> Result<()> {
        log::trace!("watcher add: {path:?}");
        let tx = self.tx.clone();
        let pending_paths = self.pending_path_events.clone();

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            // Return early if an ancestor of this path was already being watched.
            // saves a huge amount of memory
            let registrations = self.registrations.lock();
            let range = registrations.range::<Path, _>((std::ops::Bound::Unbounded, std::ops::Bound::Included(path)));

            for (watched_path, _) in range.rev() {
                if path.starts_with(watched_path.as_ref()) {
                    log::trace!("path to watch is covered by existing registration: {path:?}, {watched_path:?}");
                    return Ok(());
                }
            }
        }
        #[cfg(any(target_os = "linux"))]
        {
            if self.registrations.lock().contains_key(path) {
                log::trace!("path to watch is already watched: {path:?}");
                return Ok(());
            }
        }

        match path.try_exists() {
            Ok(true) => {}
            Ok(false) => anyhow::bail!("Path to watch does not exist: {path:?}"),
            Err(e) => anyhow::bail!("Path to watch may not exist: {path:?}: {e}"),
        }

        let cano_path = path
            .canonicalize()
            .map(|v| SanitizedPath::new_arc::<Path>(v.as_ref()))
            .ok();
        let root_path = SanitizedPath::new_arc(path);
        let path: Arc<Path> = path.into();

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let mode = notify::RecursiveMode::Recursive;
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let mode = notify::RecursiveMode::NonRecursive;

        let registration_path = path.clone();

        let registration = global({
            let debug_path = path.clone();
            let watch_path = path.clone();
            let callback_path = path.clone();
            |g| {
                g.add(watch_path, mode, move |event: &notify::Event| {
                    log::trace!("watcher ({:?}) received event: {:?}", debug_path, event);
                    let kind = match event.kind {
                        EventKind::Create(_) => Some(PathEventKind::Created),
                        EventKind::Modify(_) => Some(PathEventKind::Changed),
                        EventKind::Remove(_) => Some(PathEventKind::Removed),
                        _ => None,
                    };
                    let mut path_events = event
                        .paths
                        .iter()
                        .filter_map(|event_path| {
                            let cano_path = cano_path.clone();
                            let event_path = SanitizedPath::new(event_path);
                            if event_path.starts_with(&root_path) {
                                return Some(PathEvent {
                                    path: event_path.as_path().to_path_buf(),
                                    kind,
                                });
                            }
                            if let Some(cano) = &cano_path {
                                if event_path.starts_with(cano) {
                                    return Some(PathEvent {
                                        path: event_path.as_path().to_path_buf(),
                                        kind,
                                    });
                                }
                            }
                            return None;
                        })
                        .collect::<Vec<_>>();

                    let is_rescan_event = event.need_rescan();
                    if is_rescan_event {
                        log::warn!("filesystem watcher lost sync for {callback_path:?}");
                        path_events.retain(|p| &p.path != callback_path.as_ref());
                        path_events.push(PathEvent {
                            path: callback_path.to_path_buf(),
                            kind: Some(PathEventKind::Rescan),
                        });
                    }

                    if !path_events.is_empty() {
                        push_path_events(&tx, &pending_paths, path_events);
                    }
                })
            }
        });

        match registration {
            Ok(Ok(registration_id)) => {
                self.registrations.lock().insert(registration_path, registration_id);

                Ok(())
            }
            Err(e) | Ok(Err(e)) => {
                log::warn!("Fall back to poll watcher: {}", e,);
                return self.poll_path(&path);
            }
        }
    }

    fn remove(&self, path: &Path) -> Result<()> {
        log::trace!("remove watched path: {path:?}");
        if let Some(registration) = self.registrations.lock().remove(path) {
            global(|w| w.remove(registration))?;
        }
        if let Some(fallback) = self.poll_fallback.lock().as_ref() {
            fallback.remove(path)?;
        }
        Ok(())
    }
}

pub(crate) struct PollFsWatcher {
    watcher: Mutex<(notify::PollWatcher, Vec<Arc<Path>>)>,
}

impl PollFsWatcher {
    pub fn new(
        tx: Sender<()>,
        pending_path_events: Arc<Mutex<Vec<PathEvent>>>,
        poll_interval: Duration,
    ) -> Result<Self> {
        let config = notify::Config::default()
            .with_event_kinds(EventKindMask::CORE)
            .with_poll_interval(poll_interval);

        let watcher = notify::PollWatcher::new(
            move |event: Result<notify::Event, notify::Error>| {
                let Some(event) = event.ok().filter(|event| !matches!(event.kind, EventKind::Access(_))) else {
                    return;
                };

                let kind = match event.kind {
                    EventKind::Create(_) => Some(PathEventKind::Created),
                    EventKind::Modify(_) => Some(PathEventKind::Changed),
                    EventKind::Remove(_) => Some(PathEventKind::Removed),
                    _ => None,
                };

                let path_events = event
                    .paths
                    .iter()
                    .map(|event_path| PathEvent {
                        path: event_path.to_path_buf(),
                        kind,
                    })
                    .collect::<Vec<_>>();

                if !path_events.is_empty() {
                    push_path_events(&tx, &pending_path_events, path_events);
                }
            },
            config,
        )?;

        Ok(Self {
            watcher: Mutex::new((watcher, Vec::new())),
        })
    }
}

impl Watcher for PollFsWatcher {
    fn add(&self, path: &Path) -> Result<()> {
        use notify::Watcher;

        if self
            .watcher
            .lock()
            .1
            .iter()
            .any(|watched_path| path.starts_with(watched_path.as_ref()))
        {
            log::trace!("path to watch is covered by existing registration: {path:?}");
            return Ok(());
        }

        if !path.exists() {
            anyhow::bail!("Path to watch does not exist: {path:?}");
        }

        let mut guard = self.watcher.lock();
        guard.0.watch(path, notify::RecursiveMode::Recursive)?;
        guard.1.push(path.into());
        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<()> {
        use notify::Watcher;
        let mut guard = self.watcher.lock();
        guard.0.unwatch(path)?;
        guard.1.retain(|p| p.as_ref() != path);
        Ok(())
    }
}

fn push_path_events(
    tx: &Sender<()>,
    pending_path_events: &Arc<Mutex<Vec<PathEvent>>>,
    mut path_events: Vec<PathEvent>,
) {
    if path_events.is_empty() {
        return;
    }

    path_events.sort();
    let mut pending_paths = pending_path_events.lock();
    if pending_paths.is_empty() {
        tx.try_send(()).ok();
    }
    coalesce_pending_rescans(&mut pending_paths, &mut path_events);
    util::extend_sorted(&mut *pending_paths, path_events, usize::MAX, |a, b| a.path.cmp(&b.path));
}

fn coalesce_pending_rescans(pending_paths: &mut Vec<PathEvent>, path_events: &mut Vec<PathEvent>) {
    if !path_events
        .iter()
        .any(|event| event.kind == Some(PathEventKind::Rescan))
    {
        return;
    }

    let mut new_rescan_paths: Vec<std::path::PathBuf> = path_events
        .iter()
        .filter(|e| e.kind == Some(PathEventKind::Rescan))
        .map(|e| e.path.clone())
        .collect();
    new_rescan_paths.sort_unstable();

    let mut deduped_rescans: Vec<std::path::PathBuf> = Vec::with_capacity(new_rescan_paths.len());
    for path in new_rescan_paths {
        if deduped_rescans
            .iter()
            .any(|ancestor| path != *ancestor && path.starts_with(ancestor))
        {
            continue;
        }
        deduped_rescans.push(path);
    }

    deduped_rescans.retain(|new_path| {
        !pending_paths
            .iter()
            .any(|pending| is_covered_rescan(pending.kind, new_path, &pending.path))
    });

    if !deduped_rescans.is_empty() {
        pending_paths.retain(|pending| {
            !deduped_rescans.iter().any(|rescan_path| {
                pending.path == *rescan_path || is_covered_rescan(pending.kind, &pending.path, rescan_path)
            })
        });
    }

    path_events.retain(|event| event.kind != Some(PathEventKind::Rescan) || deduped_rescans.contains(&event.path));
}

fn is_covered_rescan(kind: Option<PathEventKind>, path: &Path, ancestor: &Path) -> bool {
    kind == Some(PathEventKind::Rescan) && path != ancestor && path.starts_with(ancestor)
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct WatcherRegistrationId(u32);

struct WatcherRegistrationState {
    callback: Arc<dyn Fn(&notify::Event) + Send + Sync>,
    path: Arc<Path>,
}

struct WatcherState {
    watchers: HashMap<WatcherRegistrationId, WatcherRegistrationState>,
    path_registrations: HashMap<Arc<Path>, u32>,
    last_registration: WatcherRegistrationId,
}

pub struct GlobalWatcher {
    state: Mutex<WatcherState>,

    // DANGER: never keep the state lock while holding the watcher lock
    // two mutexes because calling watcher.add triggers an watcher.event, which needs watchers.
    #[cfg(target_os = "macos")]
    watcher: Mutex<notify::FsEventWatcher>,
    #[cfg(target_os = "linux")]
    watcher: Mutex<notify::INotifyWatcher>,
    #[cfg(target_os = "freebsd")]
    watcher: Mutex<notify::KqueueWatcher>,
    #[cfg(target_os = "windows")]
    watcher: Mutex<notify::ReadDirectoryChangesWatcher>,
}

impl GlobalWatcher {
    #[must_use]
    fn add(
        &self,
        path: Arc<Path>,
        mode: notify::RecursiveMode,
        cb: impl Fn(&notify::Event) + Send + Sync + 'static,
    ) -> Result<WatcherRegistrationId> {
        use notify::Watcher;

        if self
            .state
            .lock()
            .path_registrations
            .get(&path)
            .map_or(true, |&count| count == 0)
        {
            self.watcher.lock().watch(&path, mode)?;
        }

        let mut state = self.state.lock();
        let id = state.last_registration;
        state.last_registration = WatcherRegistrationId(id.0 + 1);
        let registration_state = WatcherRegistrationState {
            callback: Arc::new(cb),
            path: path.clone(),
        };
        state.watchers.insert(id, registration_state);
        *state.path_registrations.entry(path).or_insert(0) += 1;

        Ok(id)
    }

    pub fn remove(&self, id: WatcherRegistrationId) {
        use notify::Watcher;
        let mut state = self.state.lock();
        let Some(registration_state) = state.watchers.remove(&id) else {
            return;
        };

        let Some(count) = state.path_registrations.get_mut(&registration_state.path) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            state.path_registrations.remove(&registration_state.path);

            drop(state);
            self.watcher.lock().unwatch(&registration_state.path).log_err();
        }
    }
}

static FS_WATCHER_INSTANCE: OnceLock<Result<GlobalWatcher, notify::Error>> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn rescan(path: &str) -> PathEvent {
        PathEvent {
            path: PathBuf::from(path),
            kind: Some(PathEventKind::Rescan),
        }
    }

    fn changed(path: &str) -> PathEvent {
        PathEvent {
            path: PathBuf::from(path),
            kind: Some(PathEventKind::Changed),
        }
    }

    struct TestCase {
        name: &'static str,
        pending_paths: Vec<PathEvent>,
        path_events: Vec<PathEvent>,
        expected_pending_paths: Vec<PathEvent>,
        expected_path_events: Vec<PathEvent>,
    }

    #[test]
    fn test_coalesce_pending_rescans() {
        let test_cases = [
            TestCase {
                name: "coalesces descendant rescans under pending ancestor",
                pending_paths: vec![rescan("/root")],
                path_events: vec![rescan("/root/child"), rescan("/root/child/grandchild")],
                expected_pending_paths: vec![rescan("/root")],
                expected_path_events: vec![],
            },
            TestCase {
                name: "new ancestor rescan replaces pending descendant rescans",
                pending_paths: vec![
                    changed("/other"),
                    rescan("/root/child"),
                    rescan("/root/child/grandchild"),
                ],
                path_events: vec![rescan("/root")],
                expected_pending_paths: vec![changed("/other")],
                expected_path_events: vec![rescan("/root")],
            },
            TestCase {
                name: "same path rescan replaces pending non-rescan event",
                pending_paths: vec![changed("/root")],
                path_events: vec![rescan("/root")],
                expected_pending_paths: vec![],
                expected_path_events: vec![rescan("/root")],
            },
            TestCase {
                name: "unrelated rescans are preserved",
                pending_paths: vec![rescan("/root-a")],
                path_events: vec![rescan("/root-b")],
                expected_pending_paths: vec![rescan("/root-a")],
                expected_path_events: vec![rescan("/root-b")],
            },
            TestCase {
                name: "batch ancestor rescan replaces descendant rescan",
                pending_paths: vec![],
                path_events: vec![rescan("/root/child"), rescan("/root")],
                expected_pending_paths: vec![],
                expected_path_events: vec![rescan("/root")],
            },
        ];

        for test_case in test_cases {
            let mut pending_paths = test_case.pending_paths;
            let mut path_events = test_case.path_events;

            coalesce_pending_rescans(&mut pending_paths, &mut path_events);

            assert_eq!(
                pending_paths, test_case.expected_pending_paths,
                "pending_paths mismatch for case: {}",
                test_case.name
            );
            assert_eq!(
                path_events, test_case.expected_path_events,
                "path_events mismatch for case: {}",
                test_case.name
            );
        }
    }
}

fn handle_event(event: Result<notify::Event, notify::Error>) {
    log::trace!("global handle event: {event:?}");
    // Filter out access events, which could lead to a weird bug on Linux after upgrading notify
    // https://github.com/zed-industries/zed/actions/runs/14085230504/job/39449448832
    let Some(event) = event
        .log_err()
        .filter(|event| !matches!(event.kind, EventKind::Access(_)))
    else {
        return;
    };
    global::<()>(move |watcher| {
        let callbacks = {
            let state = watcher.state.lock();
            state.watchers.values().map(|r| r.callback.clone()).collect::<Vec<_>>()
        };
        for callback in callbacks {
            callback(&event);
        }
    })
    .log_err();
}

pub fn global<T>(f: impl FnOnce(&GlobalWatcher) -> T) -> Result<T> {
    let result = FS_WATCHER_INSTANCE.get_or_init(|| {
        let config = notify::Config::default().with_event_kinds(EventKindMask::CORE);
        notify::RecommendedWatcher::new(handle_event, config).map(|file_watcher| GlobalWatcher {
            state: Mutex::new(WatcherState {
                watchers: Default::default(),
                path_registrations: Default::default(),
                last_registration: Default::default(),
            }),
            watcher: Mutex::new(file_watcher),
        })
    });
    match result {
        Ok(g) => Ok(f(g)),
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}
