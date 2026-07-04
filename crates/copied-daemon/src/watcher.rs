//! Wayland clipboard watcher.
//!
//! Connects to the compositor using the `wlr-data-control-unstable-v1` protocol
//! (`zwlr_data_control_manager_v1`) and listens for clipboard selection changes,
//! emitting [`ClipboardChange`] events over an `mpsc::Sender`.
//!
//! `wl-clipboard-rs` is intentionally not used here: it only exposes `copy` and
//! `paste` modules, with no support for watching selection changes.
//!
//! On Pop!_OS/COSMIC, `cosmic-comp` only advertises this protocol when the
//! `COSMIC_DATA_CONTROL_ENABLED=1` environment variable is set for the
//! compositor process. That is a system/session setup concern, not something
//! this module can control; if the protocol is unavailable we log a clear
//! error and exit.

use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use std::os::fd::AsFd;
use std::sync::mpsc::Sender;
use std::time::Duration;

use wayland_client::backend::ObjectId;
use wayland_client::globals::{registry_queue_init, BindError, GlobalError, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::{
    self, ZwlrDataControlDeviceV1,
};
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::{
    self, ZwlrDataControlOfferV1,
};

/// A clipboard content change detected on the compositor's selection.
pub enum ClipboardChange {
    /// Plain text content.
    Text(String),
    /// Image content, along with its MIME type.
    Image { bytes: Vec<u8>, mime: String },
}

/// MIME types recognized as plain text, in priority order.
const TEXT_MIME_TYPES: &[&str] = &["text/plain", "text/plain;charset=utf-8", "UTF8_STRING"];

/// MIME types recognized as images, in priority order.
const IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg"];

/// Reconnection backoff schedule, in seconds. The last entry is reused once exhausted.
const RECONNECT_BACKOFFS_SECS: &[u64] = &[1, 2, 5, 30];

/// Runs the clipboard watcher forever, emitting [`ClipboardChange`] events on `tx`.
///
/// If the very first connection attempt fails (e.g. no Wayland session, or the
/// compositor doesn't advertise `zwlr_data_control_manager_v1`), this logs a
/// clear error and exits the process rather than looping silently. Once a
/// connection has been established at least once, subsequent drops are
/// retried with an increasing backoff (1s, 2s, 5s, capped at 30s).
pub fn run(tx: Sender<ClipboardChange>) -> ! {
    let mut ever_connected = false;
    let mut backoff_idx = 0usize;

    loop {
        match watch_once(&tx, &mut ever_connected, &mut backoff_idx) {
            Ok(()) => unreachable!("watch_once only returns on error"),
            Err(err) => {
                if !ever_connected {
                    eprintln!(
                        "copied-daemon: could not start the clipboard watcher: {err}. \
                         On Pop!_OS/COSMIC, make sure COSMIC_DATA_CONTROL_ENABLED=1 is set \
                         for the compositor so it advertises the data-control protocol."
                    );
                    std::process::exit(1);
                }

                let delay_secs =
                    RECONNECT_BACKOFFS_SECS[backoff_idx.min(RECONNECT_BACKOFFS_SECS.len() - 1)];
                eprintln!(
                    "copied-daemon: Wayland clipboard watcher disconnected ({err}); \
                     reconnecting in {delay_secs}s"
                );
                std::thread::sleep(Duration::from_secs(delay_secs));
                backoff_idx += 1;
            }
        }
    }
}

/// Error connecting to or communicating over the Wayland data-control protocol.
#[derive(Debug)]
enum WatchError {
    Connect(wayland_client::ConnectError),
    Globals(GlobalError),
    Bind(&'static str, BindError),
    Dispatch(wayland_client::DispatchError),
    Pipe(std::io::Error),
    Finished,
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatchError::Connect(err) => write!(f, "failed to connect to Wayland: {err}"),
            WatchError::Globals(err) => write!(f, "failed to list Wayland globals: {err}"),
            WatchError::Bind(iface, err) => {
                write!(f, "failed to bind Wayland global {iface}: {err}")
            }
            WatchError::Dispatch(err) => write!(f, "Wayland dispatch error: {err}"),
            WatchError::Pipe(err) => write!(f, "clipboard content transfer failed: {err}"),
            WatchError::Finished => write!(f, "data-control device was invalidated"),
        }
    }
}

/// Which kind of clipboard content a negotiated MIME type represents.
enum Kind {
    Text,
    Image,
}

/// Per-connection watcher state.
struct AppState {
    /// MIME types offered by each still-live offer, keyed by Wayland object id.
    offers: HashMap<ObjectId, Vec<String>>,
    /// The offer backing the current regular-clipboard selection, if any.
    current_offer: Option<ZwlrDataControlOfferV1>,
    /// An offer that just became the selection and is waiting to be read.
    pending: Option<ZwlrDataControlOfferV1>,
    /// Set when the compositor invalidated our data-control device.
    finished: bool,
}

/// Connects once, watches for clipboard changes until the connection drops, and
/// reports the outcome. Only returns `Err`; success is "ran until disconnected".
fn watch_once(
    tx: &Sender<ClipboardChange>,
    ever_connected: &mut bool,
    backoff_idx: &mut usize,
) -> Result<(), WatchError> {
    let conn = Connection::connect_to_env().map_err(WatchError::Connect)?;
    let (globals, mut queue) =
        registry_queue_init::<AppState>(&conn).map_err(WatchError::Globals)?;
    let qh: QueueHandle<AppState> = queue.handle();

    let manager: ZwlrDataControlManagerV1 = globals
        .bind(&qh, 1..=2, ())
        .map_err(|err| WatchError::Bind("zwlr_data_control_manager_v1", err))?;
    let seat: WlSeat = globals
        .bind(&qh, 1..=1, ())
        .map_err(|err| WatchError::Bind("wl_seat", err))?;

    let mut state = AppState {
        offers: HashMap::new(),
        current_offer: None,
        pending: None,
        finished: false,
    };

    let _device: ZwlrDataControlDeviceV1 = manager.get_data_device(&seat, &qh, ());

    // We got this far: the protocol is supported and the device was created.
    *ever_connected = true;
    *backoff_idx = 0;

    loop {
        queue
            .blocking_dispatch(&mut state)
            .map_err(WatchError::Dispatch)?;

        if state.finished {
            return Err(WatchError::Finished);
        }

        if let Some(offer) = state.pending.take() {
            process_offer(&mut queue, &mut state, offer, tx)?;
        }
    }
}

/// Negotiates a MIME type for `offer`, reads its content, and emits the
/// resulting [`ClipboardChange`]. Offers with no recognized MIME type are
/// silently dropped, per the module's acceptance criteria.
fn process_offer(
    queue: &mut EventQueue<AppState>,
    state: &mut AppState,
    offer: ZwlrDataControlOfferV1,
    tx: &Sender<ClipboardChange>,
) -> Result<(), WatchError> {
    let id = offer.id();
    let mimes = state.offers.remove(&id).unwrap_or_default();

    let chosen = TEXT_MIME_TYPES
        .iter()
        .find(|wanted| mimes.iter().any(|offered| offered == *wanted))
        .map(|mime| (Kind::Text, (*mime).to_string()))
        .or_else(|| {
            IMAGE_MIME_TYPES
                .iter()
                .find(|wanted| mimes.iter().any(|offered| offered == *wanted))
                .map(|mime| (Kind::Image, (*mime).to_string()))
        });

    let Some((kind, mime)) = chosen else {
        offer.destroy();
        return Ok(());
    };

    let (mut reader, writer) = std::io::pipe().map_err(WatchError::Pipe)?;
    offer.receive(mime.clone(), writer.as_fd());
    // Drop our copy of the write end so we observe EOF once the source client
    // (which received its own duplicated fd) finishes writing and closes it.
    drop(writer);

    // A plain flush() can race the compositor forwarding the receive request;
    // a roundtrip ensures the request has actually been processed before we
    // block reading from the pipe.
    queue.roundtrip(state).map_err(WatchError::Dispatch)?;

    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(WatchError::Pipe)?;

    offer.destroy();

    let change = match kind {
        Kind::Text => ClipboardChange::Text(String::from_utf8_lossy(&bytes).into_owned()),
        Kind::Image => ClipboardChange::Image { bytes, mime },
    };

    // If the receiving end went away, there's nothing more we can do; keep watching.
    let _ = tx.send(change);

    Ok(())
}

impl Dispatch<WlRegistry, GlobalListContents> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: wayland_client::protocol::wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Dynamic global add/remove after startup is irrelevant to us: we only
        // need the manager and seat resolved once, at connection time.
    }
}

delegate_noop!(AppState: ignore WlSeat);
delegate_noop!(AppState: ignore ZwlrDataControlManagerV1);

impl Dispatch<ZwlrDataControlDeviceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                state.offers.insert(id.id(), Vec::new());
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                if let Some(old) = state.current_offer.take() {
                    state.offers.remove(&old.id());
                    old.destroy();
                }
                state.current_offer = id.clone();
                state.pending = id;
            }
            // Primary selection (middle-click paste) is out of scope for this
            // watcher; discard the offer without reading it.
            zwlr_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                state.offers.remove(&offer.id());
                offer.destroy();
            }
            zwlr_data_control_device_v1::Event::Finished => {
                state.finished = true;
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(AppState, ZwlrDataControlDeviceV1, [
        zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.offers.entry(proxy.id()).or_default().push(mime_type);
        }
    }
}
