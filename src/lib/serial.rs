//! Items related to receiving CBM 8032 frame data over serial.

use crate::fps::Fps;
use crate::vis;
use serialport::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{self, AtomicBool};
use std::sync::{mpsc, Arc};

const BAUD_RATE: u32 = 3_000_000;
const BUFFER_LEN: usize = 41;
const DATA_PER_BUFFER: u8 = 40;
const NON_DATA_BUFFER_N: u8 = 51;

/// A handle to the receiving serial thread.
pub struct Handle {
    is_closed: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
    rx: ChannelRx,
    last_recorded_frame_hz: RefCell<FrameHz>,
    port_info: SerialPortInfo,
}

type Message = (vis::Cbm8032Frame, FrameHz);
type ChannelRx = mpsc::Receiver<Message>;
type ChannelTx = mpsc::Sender<Message>;
type SerialPortObj = dyn SerialPort;

enum State {
    CountingZeroes(u8),
    InSync {
        last_buffer_n: u8,
        buffer: Vec<u8>,
        vis_frame_data: Vec<u8>,
    },
}

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
    settings.timeout = std::time::Duration::from_millis(17);
    serialport::open_with_settings(&name, &settings)
}

// The same as `Read::read` but ignores `TimedOut` and `WouldBlock` errors.
fn read_from(port: &mut Box<SerialPortObj>, buffer: &mut [u8]) -> usize {
    match port.read(buffer) {
        Err(e) => match e.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => 0,
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

// Open the serial port and run the read loop.
fn run(mut port: Box<SerialPortObj>, vis_frame_tx: ChannelTx, is_closed: Arc<AtomicBool>) {
    let mut state = State::CountingZeroes(0);
    let mut buffer = vec![0u8; 1_000];
    let fps = Fps::default();
    while !is_closed.load(atomic::Ordering::Relaxed) {
        let len = read_from(&mut port, &mut buffer[..]);
        let mut slice = &buffer[..len];
        while !slice.is_empty() {
            match state {
                // If we're counting zeroes, check for another.
                State::CountingZeroes(ref mut n_zeroes) => {
                    match slice[0] {
                        0 => *n_zeroes += 1,
                        _ => *n_zeroes = 0,
                    }
                    slice = &slice[1..];
                    if *n_zeroes == DATA_PER_BUFFER {
                        state = State::InSync {
                            last_buffer_n: 0,
                            buffer: Vec::with_capacity(BUFFER_LEN),
                            vis_frame_data: Vec::with_capacity(vis::CBM_8032_FRAME_DATA_LEN),
                        };
                    }
                }

                // If we're synchronised, read until the next full buffer.
                State::InSync {
                    ref mut last_buffer_n,
                    ref mut buffer,
                    ref mut vis_frame_data,
                } => {
                    if buffer.len() < BUFFER_LEN {
                        let remaining = BUFFER_LEN - buffer.len();
                        let end = std::cmp::min(remaining, slice.len());
                        buffer.extend(slice[..end].iter().cloned());
                        slice = &slice[end..];
                        continue;
                    }

                    // If this is the 0th buffer, skip it.
                    let buffer_n = slice[0];
                    slice = &slice[1..];
                    if buffer_n == NON_DATA_BUFFER_N {
                        *last_buffer_n = buffer_n;
                        // If the last buffer was 51, this is the zeroed sync frame.
                        if *last_buffer_n == NON_DATA_BUFFER_N {
                            *last_buffer_n = 0;
                        // Otherwise this frame contains the graphics mode. The frame is ready.
                        } else {
                            let mode = match buffer[0] {
                                0 => vis::Cbm8032FrameMode::Text,
                                _ => vis::Cbm8032FrameMode::Graphics,
                            };
                            let mut data = Box::new([0u8; vis::CBM_8032_FRAME_DATA_LEN]);
                            data.copy_from_slice(vis_frame_data);
                            buffer.clear();
                            let frame = vis::Cbm8032Frame::new(mode, data);

                            // Sample the rate at which serial data is producing frames.
                            fps.sample();
                            let avg = fps.avg();
                            let min = fps.min();
                            let max = fps.max();
                            let hz = FrameHz { avg, min, max };

                            if vis_frame_tx.send((frame, hz)).is_err() {
                                eprintln!("lost connecton to main thread, closing serial thread");
                                return;
                            }
                        }
                        continue;
                    }

                    // Check we're still synchronised.
                    if buffer_n - 1 != *last_buffer_n {
                        eprintln!("lost serial sync, attempting re-sync");
                        state = State::CountingZeroes(0);
                        continue;
                    } else {
                        *last_buffer_n = buffer_n;
                    }

                    vis_frame_data.extend(buffer.drain(..));
                }
            }
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
        .spawn(move || {
            run(port, tx, is_closed2)
        })
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
