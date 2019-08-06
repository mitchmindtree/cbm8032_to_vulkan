//! Items related to receiving CBM 8032 frame data over serial.

use crate::fps::Fps;
use crate::vis;
use serialport::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{self, AtomicBool};
use std::sync::{mpsc, Arc};

const BAUD_RATE: u32 = 1_500_000;
const DATA_PER_BUFFER: u32 = 40;
const TOTAL_BUFFERS_PER_FRAME: u32 = 51;

/// A handle to the receiving serial thread.
pub struct Handle {
    is_closed: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
    rx: ChannelRx,
    last_recorded_frame_hz: RefCell<FrameHz>,
    port_info: SerialPortInfo,
}

enum State {
    CountingZeros,
    InSync,
}

struct ReceiverContext {
    rx_buffer: [u8; 256],
    rx_buffer_index: u32,
    rx_buffer_count: u32,
    state: State,
    bufnum: u32,
    count: u32,
    buffer: [u8; 40],
    screen_buffer: Box<vis::Cbm8032FrameData>,
    graphic: vis::Cbm8032FrameMode,
}

fn init_receiver_context() -> ReceiverContext {
    ReceiverContext {
        rx_buffer: [0u8; 256],
        rx_buffer_index: 0,
        rx_buffer_count: 0,
        bufnum: 0,
        count: 0,
        state: State::CountingZeros,
        buffer: [0u8; 40],
        screen_buffer: Box::new([0u8; vis::CBM_8032_FRAME_DATA_LEN]),
        graphic: vis::Cbm8032FrameMode::Graphics,
    }
}

type Message = (vis::Cbm8032Frame, FrameHz);
type ChannelRx = mpsc::Receiver<Message>;
type ChannelTx = mpsc::Sender<Message>;
type SerialPortObj = dyn SerialPort;

/// The rate at which the serial stream is producing frames.
#[derive(Clone, Copy, Default)]
pub struct FrameHz {
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

impl Handle {
    /// Checks the queue for a pending frame and returns it.
    pub fn try_recv_frame(&self) -> Option<vis::Cbm8032Frame> {
        if let Some((frame, hz)) = self.rx.try_iter().last() {
            *self.last_recorded_frame_hz.borrow_mut() = hz;
            return Some(frame);
        }
        None
    }

    /// Produces the last frame sending rate sent by the serial thread.
    pub fn frame_hz(&self) -> FrameHz {
        *self.last_recorded_frame_hz.borrow()
    }

    /// Close the receiving thread.
    pub fn close(self) {
        self.is_closed.store(true, atomic::Ordering::SeqCst);
        if let Err(e) = self.thread.join() {
            eprintln!("failed to join serial thread: {:?}", e);
        }
    }

    /// Information about the connected serial port.
    pub fn port_info(&self) -> &SerialPortInfo {
        &self.port_info
    }
}

fn find_usb_port() -> Result<Option<SerialPortInfo>, serialport::Error> {
    let infos = serialport::available_ports()?;
    let info = infos
        .into_iter()
        .filter(|info| match info.port_type {
            serialport::SerialPortType::UsbPort(_) => true,
            _ => false,
        })
        .next();
    Ok(info)
}

fn open_port(name: &str) -> Result<Box<SerialPortObj>, serialport::Error> {
    let mut settings = SerialPortSettings::default();
    settings.baud_rate = BAUD_RATE.into();
    settings.timeout = std::time::Duration::from_secs(1);
    serialport::open_with_settings(&name, &settings)
}

// The same as `Read::read` but ignores `TimedOut` and `WouldBlock` errors.
fn read_from(port: &mut Box<SerialPortObj>, buffer: &mut [u8]) -> usize {
    match port.read(buffer) {
        Err(e) => match e.kind() {
            std::io::ErrorKind::TimedOut => {
                eprintln!("no serial data received in the last second");
                0
            }
            std::io::ErrorKind::WouldBlock => 0,
            _ => {
                eprintln!(
                    "An error occurred while reading from the serial port: {}",
                    e
                );
                0
            }
        },
        Ok(len) => len,
    }
}

fn byte_to_mode(byte: u8) -> vis::Cbm8032FrameMode {
    match byte {
        0 => vis::Cbm8032FrameMode::Graphics,
        _ => vis::Cbm8032FrameMode::Text,
    }
}

fn handle_received_buffer(context: &mut ReceiverContext) {
    if context.bufnum > 0 {
        if context.bufnum < TOTAL_BUFFERS_PER_FRAME {
            let bufidx = context.bufnum - 1;
            let screen_start = (bufidx * DATA_PER_BUFFER) as usize;
            let screen_end = screen_start + DATA_PER_BUFFER as usize;
            let screen_slice = &mut context.screen_buffer[screen_start..screen_end];
            screen_slice.copy_from_slice(&context.buffer);
        } else {
            context.graphic = byte_to_mode(context.buffer[0]);
        }
    }
}

fn handle_sync_loss(context: &ReceiverContext, byte: u8) {
    eprintln!(
        "out of sync at bufnum {} count {} - received {}\n",
        context.bufnum, context.count, byte
    );
}

fn handle_received_byte(context: &mut ReceiverContext, byte: u8) -> bool {
    let mut screen_complete = false;
    match context.state {
        State::CountingZeros => {
            if byte == 0 {
                context.count += 1;
                if context.count == 41 {
                    context.state = State::InSync;
                    context.bufnum = 1;
                    context.count = 0;
                }
            } else {
                context.count = 0;
            }
        }
        State::InSync => {
            if context.count < 40 {
                context.buffer[context.count as usize] = byte;
                context.count += 1;
            } else {
                if byte == context.bufnum as u8 {
                    handle_received_buffer(context);
                    context.bufnum += 1;
                    context.count = 0;
                    if context.bufnum == 52 {
                        context.state = State::CountingZeros;
                        screen_complete = true;
                    }
                } else {
                    handle_sync_loss(context, byte);
                    context.state = State::CountingZeros;
                    context.count = 0;
                }
            }
        }
    }
    screen_complete
}

fn receive_screen(port: &mut Box<SerialPortObj>, context: &mut ReceiverContext) {
    loop {
        if context.rx_buffer_index == context.rx_buffer_count {
            context.rx_buffer_index = 0;
            context.rx_buffer_count = read_from(port, &mut context.rx_buffer) as _;
        } else {
            let ix = context.rx_buffer_index;
            context.rx_buffer_index += 1;
            let received_byte = context.rx_buffer[ix as usize];
            if handle_received_byte(context, received_byte) {
                return;
            }
        }
    }
}

// Open the serial port and run the read loop.
fn run(mut port: Box<SerialPortObj>, vis_frame_tx: ChannelTx, is_closed: Arc<AtomicBool>) {
    let fps = Fps::default();
    let mut context = init_receiver_context();
    while !is_closed.load(atomic::Ordering::Relaxed) {
        receive_screen(&mut port, &mut context);

        // Construct the frame.
        let frame = vis::Cbm8032Frame::new(context.graphic, context.screen_buffer.clone());

        // Sample the rate at which serial data is producing frames.
        fps.sample();
        let avg = fps.avg();
        let min = fps.min();
        let max = fps.max();
        let hz = FrameHz { avg, min, max };

        // Send the frame to the main thread.
        if vis_frame_tx.send((frame, hz)).is_err() {
            eprintln!("lost connecton to main thread, closing serial thread");
            return;
        }
    }
}

/// Spawn a thread for receiving serial data.
pub fn spawn() -> Result<Handle, serialport::Error> {
    let is_closed = Arc::new(AtomicBool::new(false));
    let is_closed2 = is_closed.clone();
    let (tx, rx) = mpsc::channel();
    let info = match find_usb_port()? {
        Some(info) => info,
        None => {
            let desc = "no available serial USB ports".to_string();
            let kind = serialport::ErrorKind::NoDevice;
            return Err(serialport::Error::new(kind, desc));
        }
    };
    let port = open_port(&info.port_name)?;
    let thread = std::thread::Builder::new()
        .name("serial_rx_thread".into())
        .spawn(move || run(port, tx, is_closed2))
        .expect("failed to spawn serial rx thread");
    let last_recorded_frame_hz = RefCell::new(FrameHz::default());
    Ok(Handle {
        is_closed,
        rx,
        thread,
        last_recorded_frame_hz,
        port_info: info,
    })
}
