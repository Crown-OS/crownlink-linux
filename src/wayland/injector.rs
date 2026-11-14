//! Feeding a paired device's pointer, keyboard and touch into this computer: into the user's
//! seat through the crownos-input-v1 injector, or, for a guest working on an output of their
//! own, into a seat of the guest's own through crownos-guest-seat-v1.

use std::sync::{Arc, Mutex};

use crownos_protocols::guest_seat::v1::client::crownos_guest_seat_manager_v1::{
    self, CrownosGuestSeatManagerV1,
};
use crownos_protocols::guest_seat::v1::client::crownos_guest_seat_v1::{
    self as guest_seat, CrownosGuestSeatV1,
};
use crownos_protocols::input::v1::client::crownos_input_injector_v1::{
    self, Axis, ButtonState, CrownosInputInjectorV1, KeyState,
};
use crownos_protocols::input::v1::client::crownos_input_manager_v1::{self, CrownosInputManagerV1};
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle};

use super::input_types::{split_micros, ScrollAxis};
use super::outputs::{logical_point, track_outputs, Outputs};
use super::worker::{WorkerHandle, WorkerState};
use super::WaylandError;

/// One input event; absolute positions are fractions (0.0 to 1.0) of the target output, so the
/// sender needs no knowledge of its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InjectedEvent {
    PointerMotion {
        dx: f64,
        dy: f64,
    },
    PointerTo {
        x: f64,
        y: f64,
    },
    Button {
        code: u32,
        pressed: bool,
    },
    Axis {
        axis: ScrollAxis,
        value: f64,
        discrete_120: i32,
    },
    Key {
        code: u32,
        pressed: bool,
    },
    TouchDown {
        id: i32,
        x: f64,
        y: f64,
    },
    TouchMotion {
        id: i32,
        x: f64,
        y: f64,
    },
    TouchUp {
        id: i32,
    },
    TouchCancel,
}

/// Events that belong to one hardware event, applied together.
#[derive(Debug, Clone, PartialEq)]
pub struct InputFrame {
    pub events: Vec<InjectedEvent>,
    pub time_us: u64,
}

/// The target output's logical size as the injector last saw it.
type SharedSize = Arc<Mutex<Option<(f64, f64)>>>;

/// The compositor limits a guest's name to this many bytes.
const MAX_GUEST_NAME_LEN: usize = 32;
/// The touch point a guest seat follows with its pointer, since a seat has no touch.
const PRIMARY_TOUCH: i32 = 0;
const BUTTON_LEFT: u32 = 0x110;

/// Whose seat the input goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SeatChoice {
    User { target: Option<String> },
    Guest { output: String, name: String },
}

#[derive(Debug)]
pub struct Injector {
    worker: WorkerHandle<InputFrame>,
    target_size: SharedSize,
}

impl Injector {
    /// Injects into the first seat; absolute events land on the output named `target`, or on
    /// the first output. Blocks for one roundtrip, so call it from a pipeline thread.
    ///
    /// # Errors
    ///
    /// Fails without a compositor offering crownos-input-v1 and a seat.
    pub fn connect(target: Option<String>) -> Result<Self, WaylandError> {
        Self::spawn(SeatChoice::User { target })
    }

    /// Injects into a seat of the guest's own, confined to `output`, so the user's pointer and
    /// focus stay where they are. A compositor without crownos-guest-seat-v1 gets the input on
    /// the user's seat instead. Blocks for one roundtrip, so call it from a pipeline thread.
    ///
    /// # Errors
    ///
    /// Fails without a compositor offering either protocol and a seat.
    pub fn connect_guest(output: String, guest_name: &str) -> Result<Self, WaylandError> {
        Self::spawn(SeatChoice::Guest {
            output,
            name: shown_guest_name(guest_name),
        })
    }

    fn spawn(choice: SeatChoice) -> Result<Self, WaylandError> {
        let target_size = SharedSize::default();
        let shared = Arc::clone(&target_size);
        let worker = WorkerHandle::spawn_blocking("crownlink-injector", move |globals, queue| {
            InjectorState::bind(globals, queue, choice, shared)
        })?;
        Ok(Self {
            worker,
            target_size,
        })
    }

    /// The target output's logical size, once the first frame was injected.
    pub fn target_size(&self) -> Option<(f64, f64)> {
        self.target_size.lock().ok().and_then(|size| *size)
    }

    /// # Errors
    ///
    /// Fails once the injector's thread has stopped, as after the compositor revoked it.
    pub fn inject(&self, frame: InputFrame) -> Result<(), WaylandError> {
        self.worker.send(frame)
    }
}

/// Where injected events go.
enum Sink {
    User(CrownosInputInjectorV1),
    Guest {
        seat: CrownosGuestSeatV1,
        /// The touch point the pointer follows, while one is down.
        touching: bool,
    },
}

struct InjectorState {
    outputs: Outputs,
    sink: Sink,
    target: Option<String>,
    target_size: SharedSize,
    revoked: bool,
}

track_outputs!(InjectorState, outputs);

impl InjectorState {
    fn bind(
        globals: &GlobalList,
        queue: &QueueHandle<Self>,
        choice: SeatChoice,
        target_size: SharedSize,
    ) -> Result<Self, WaylandError> {
        let outputs = Outputs::bind_existing(globals, queue);
        let (sink, target) = match choice {
            SeatChoice::Guest { output, name } => {
                match globals.bind::<CrownosGuestSeatManagerV1, _, _>(queue, 1..=1, ()) {
                    Ok(manager) => {
                        let seat = manager.create_seat(name, output.clone(), queue, ());
                        let sink = Sink::Guest {
                            seat,
                            touching: false,
                        };
                        (sink, Some(output))
                    }
                    Err(error) => {
                        tracing::info!(%error, "no guest seats here; the guest uses the user's seat");
                        (user_sink(globals, queue)?, Some(output))
                    }
                }
            }
            SeatChoice::User { target } => (user_sink(globals, queue)?, target),
        };
        Ok(Self {
            outputs,
            sink,
            target,
            target_size,
            revoked: false,
        })
    }

    fn inject(&mut self, event: InjectedEvent) {
        let target = self.outputs.find(self.target.as_deref());
        let absolute =
            |x, y| target.map(|output| (output, logical_point((x, y), output.logical_size())));
        let injector = match &mut self.sink {
            Sink::User(injector) => injector,
            Sink::Guest { seat, touching } => {
                let local = |x, y| absolute(x, y).map(|(_, point)| point);
                return drive_guest(seat, touching, event, local);
            }
        };
        match event {
            InjectedEvent::PointerMotion { dx, dy } => injector.pointer_motion(dx, dy),
            InjectedEvent::PointerTo { x, y } => {
                if let Some((output, (x, y))) = absolute(x, y) {
                    injector.pointer_motion_absolute(&output.output, x, y);
                }
            }
            InjectedEvent::Button { code, pressed } => injector.button(
                code,
                if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
            ),
            InjectedEvent::Axis {
                axis,
                value,
                discrete_120,
            } => injector.axis(
                match axis {
                    ScrollAxis::Vertical => Axis::VerticalScroll,
                    ScrollAxis::Horizontal => Axis::HorizontalScroll,
                },
                value,
                discrete_120,
            ),
            InjectedEvent::Key { code, pressed } => injector.key(
                code,
                if pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                },
            ),
            InjectedEvent::TouchDown { id, x, y } => {
                if let Some((output, (x, y))) = absolute(x, y) {
                    injector.touch_down(id, &output.output, x, y);
                }
            }
            InjectedEvent::TouchMotion { id, x, y } => {
                if let Some((_, (x, y))) = absolute(x, y) {
                    injector.touch_motion(id, x, y);
                }
            }
            InjectedEvent::TouchUp { id } => injector.touch_up(id),
            InjectedEvent::TouchCancel => injector.touch_cancel(),
        }
    }

    fn frame(&self, time_us: u64) {
        match &self.sink {
            Sink::User(injector) => {
                let (hi, lo) = split_micros(time_us);
                injector.frame(hi, lo);
            }
            Sink::Guest { seat, .. } => {
                seat.frame(u32::try_from(time_us / MICROS_PER_MILLI).unwrap_or_default());
            }
        }
    }
}

const MICROS_PER_MILLI: u64 = 1_000;

fn user_sink(
    globals: &GlobalList,
    queue: &QueueHandle<InjectorState>,
) -> Result<Sink, WaylandError> {
    let manager: CrownosInputManagerV1 = globals.bind(queue, 1..=1, ())?;
    let seat: WlSeat = globals.bind(queue, 1..=9, ())?;
    Ok(Sink::User(manager.create_injector(&seat, queue, ())))
}

/// A guest seat has a pointer and a keyboard; the primary touch point drives the pointer and
/// its left button, and other touch points are dropped.
fn drive_guest(
    seat: &CrownosGuestSeatV1,
    touching: &mut bool,
    event: InjectedEvent,
    local: impl Fn(f64, f64) -> Option<(f64, f64)>,
) {
    let button = |pressed: bool| {
        seat.button(
            BUTTON_LEFT,
            if pressed {
                guest_seat::ButtonState::Pressed
            } else {
                guest_seat::ButtonState::Released
            },
        );
    };
    match event {
        InjectedEvent::PointerMotion { dx, dy } => seat.pointer_motion(dx, dy),
        InjectedEvent::PointerTo { x, y }
        | InjectedEvent::TouchMotion {
            id: PRIMARY_TOUCH,
            x,
            y,
        } => {
            if let Some((x, y)) = local(x, y) {
                seat.pointer_motion_absolute(x, y);
            }
        }
        InjectedEvent::Button { code, pressed } => seat.button(
            code,
            if pressed {
                guest_seat::ButtonState::Pressed
            } else {
                guest_seat::ButtonState::Released
            },
        ),
        InjectedEvent::Axis { axis, value, .. } => seat.axis(
            match axis {
                ScrollAxis::Vertical => guest_seat::Axis::VerticalScroll,
                ScrollAxis::Horizontal => guest_seat::Axis::HorizontalScroll,
            },
            value,
        ),
        InjectedEvent::Key { code, pressed } => seat.key(
            code,
            if pressed {
                guest_seat::KeyState::Pressed
            } else {
                guest_seat::KeyState::Released
            },
        ),
        InjectedEvent::TouchDown {
            id: PRIMARY_TOUCH,
            x,
            y,
        } => {
            if let Some((x, y)) = local(x, y) {
                seat.pointer_motion_absolute(x, y);
                button(true);
                *touching = true;
            }
        }
        InjectedEvent::TouchUp { id: PRIMARY_TOUCH } | InjectedEvent::TouchCancel => {
            if std::mem::take(touching) {
                button(false);
            }
        }
        InjectedEvent::TouchDown { .. }
        | InjectedEvent::TouchMotion { .. }
        | InjectedEvent::TouchUp { .. } => {}
    }
}

/// The compositor takes at most 32 bytes and no control characters.
fn shown_guest_name(name: &str) -> String {
    let printable: String = name
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    let mut end = printable.len().min(MAX_GUEST_NAME_LEN);
    while !printable.is_char_boundary(end) {
        end -= 1;
    }
    match printable[..end].trim() {
        "" => "guest".to_owned(),
        shown => shown.to_owned(),
    }
}

impl WorkerState for InjectorState {
    type Command = InputFrame;

    fn apply(&mut self, frame: InputFrame, _: &QueueHandle<Self>) {
        frame.events.iter().for_each(|event| self.inject(*event));
        self.frame(frame.time_us);
        let size = self
            .outputs
            .find(self.target.as_deref())
            .map(|output| output.logical_size());
        if let Ok(mut shared) = self.target_size.lock() {
            *shared = size;
        }
    }

    fn is_finished(&self) -> bool {
        self.revoked
    }
}

impl Dispatch<CrownosInputInjectorV1, ()> for InjectorState {
    fn event(
        state: &mut Self,
        _: &CrownosInputInjectorV1,
        event: crownos_input_injector_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let crownos_input_injector_v1::Event::Revoked = event {
            state.revoked = true;
        }
    }
}

impl Dispatch<CrownosInputManagerV1, ()> for InjectorState {
    fn event(
        _: &mut Self,
        _: &CrownosInputManagerV1,
        _: crownos_input_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for InjectorState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<CrownosGuestSeatManagerV1, ()> for InjectorState {
    fn event(
        _: &mut Self,
        _: &CrownosGuestSeatManagerV1,
        _: crownos_guest_seat_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<CrownosGuestSeatV1, ()> for InjectorState {
    fn event(
        state: &mut Self,
        _: &CrownosGuestSeatV1,
        event: guest_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            guest_seat::Event::Created { seat } => tracing::debug!(seat, "the guest has a seat"),
            guest_seat::Event::Revoked { reason } => {
                tracing::info!(?reason, "the guest's seat was taken away");
                state.revoked = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_names_fit_the_compositor() {
        assert_eq!(shown_guest_name("desk"), "desk");
        assert_eq!(shown_guest_name("a\u{7}b"), "ab");
        assert_eq!(shown_guest_name("  "), "guest");
        let long = "é".repeat(40);
        let shown = shown_guest_name(&long);
        assert!(shown.len() <= MAX_GUEST_NAME_LEN);
        assert!(shown.chars().all(|character| character == 'é'));
    }
}
