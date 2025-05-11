//module for the calculation of pi

use core::f64;

use ciborium::value;
use rug::{Float, Integer, Rational};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, SystemTime};
use std::{
    i64,
    io::{stdout, Write},
    net::{TcpListener, TcpStream},
    thread::sleep,
};

#[derive(Serialize, Deserialize, Debug)]
struct ComputeResult {
    result: (Integer, Integer, Integer),
}

use crossbeam_channel::unbounded;
use std::io::{self, Take};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, Mutex};

use std::thread;

//for possible compatibility issues
const API_VERSION: usize = 4;

//used to send the tasks for the spokes
#[derive(Serialize, Deserialize, Debug)]
enum TaskPass {
    Range(i128, i128), //a range of points to calculate
    Compute((Integer, Integer, Integer), (Integer, Integer, Integer)),
    Result((Integer, Integer, Integer)), //finilazation chunk of data
}

struct Connection {
    socket: TcpStream,
    threads: u16,
    id: usize,l
    tasks: usize
}

#[derive(Deserialize)]
struct SystemInfo {
    api_version: usize,
    cores: usize,
}

//an binay split algorithem
fn bin_split(a: i128, b: i128) -> (rug::Integer, rug::Integer, rug::Integer) {
    if b == a + 1 {
        let pab = Integer::from(-(6 * a - 5) * (2 * a - 1) * (6 * a - 1));
        let qab = Integer::from(10939058860032000 * a.pow(3));
        let rab = Integer::from(&pab * (545140134 * a + 13591409));
        (pab, qab, rab)
    } else {
        let m = (a + b) / 2;
        let (pam, qam, ram) = bin_split(a, m);
        let (pmb, qmb, rmb) = bin_split(m, b);

        let pab = &pam * pmb;
        let qab = qam * &qmb;
        let rab = qmb * ram + pam * rmb;
        (pab, qab, rab)
    }
}

//used to store the result and the ranges that it must compute with.
struct BinaryReconstruction {
    input: (i128, i128),
}

fn fast_bin_split(
    n: i128,
    task_dispatch: &crossbeam_channel::Sender<TaskPass>,
    task_return: &crossbeam_channel::Receiver<TaskPass>,
    chunksize: usize,
) -> (rug::Integer, rug::Integer, rug::Integer) {
    //makes shure the tasks can be sent out
    let mut iterations = n;
    loop {
        iterations += iterations % 2;
        iterations += iterations % chunksize as i128;
        if iterations % 2 == 0 {
            break;
        }
    }
    let chunks = iterations as usize / chunksize;

    //splits the cases with the desired lengths.
    let mut tasks: Vec<TaskPass> = vec![];

    for i in 0..chunks {
        if i == 0 {
            tasks.push(TaskPass::Range(
                (i * chunks) as i128 + 1,
                ((i + 1) * chunks) as i128,
            ));
        } else {
            tasks.push(TaskPass::Range(
                (i * chunks) as i128,
                ((i + 1) * chunks) as i128,
            ));
        }
    }

    let mut compute_constructor = None;
    let mut dispatched_tasks = 0;
    loop {
        if !task_dispatch.is_full() && tasks.len() > 1 {
            let task = tasks.pop().unwrap();
            task_dispatch.send(task).unwrap();
            dispatched_tasks += 1;
        }

        if !task_return.is_empty() {
            println!("got something");
            compute_constructor = match compute_constructor {
                None => Some(task_return.recv().unwrap()),
                Some(value) => {
                    let a = match value {
                        TaskPass::Result(value) => value,
                        _ => panic!("unkown retrurn"),
                    };
                    let b = match task_return.recv().unwrap() {
                        TaskPass::Result(value) => value,
                        _ => panic!("unkown retrurn"),
                    };
                    tasks.push(TaskPass::Compute(a, b));
                    None
                }
            };
            dispatched_tasks -= 1;
        }
        if dispatched_tasks <= 0 {
            break;
        }
    }

    println!("done!");

    match compute_constructor.expect("expected compute constructor to be something") {
        TaskPass::Result(value) => value,
        _ => panic!("incrorrect value in compute constructor"),
    }
}

//runs the chudnovsky algorithem where n is the percision number
fn chudnovsky(
    n: i128,
    task_dispatch: &crossbeam_channel::Sender<TaskPass>,
    task_return: &crossbeam_channel::Receiver<TaskPass>,
    chunk_size: usize,
) -> rug::Float {
    let (_p1n, q1n, r1n) = fast_bin_split(n / 14, task_dispatch, task_return, chunk_size);
    let bits = (n as f64 * f64::consts::LOG2_10 + 16.0).ceil() as u32;

    let c = Float::with_val(bits, 426880);
    let sqrt05 = Float::with_val(bits, 10005).sqrt();

    // build the rational (13591409·q + r) / q
    let denom_rat = Rational::from((13591409 * q1n.clone() + r1n, q1n));
    let denom_f = Float::with_val(bits, denom_rat);

    c * sqrt05 / denom_f
}

#[derive(Serialize, Clone)]
pub enum PiCalcStatus {
    Running,
    Stopped,
    Quit,
    Init,
}

#[derive(Serialize, Clone)]
pub struct SpokeInfo {
    id: usize,
    cores: usize,
}

#[derive(Serialize, Clone)]
pub struct PiCalcUpdate {
    pub spokes: Vec<SpokeInfo>,
    pub status: PiCalcStatus,
    pub progres: usize,
    pub last_20: Option<String>,
    pub duration: Option<f64>,
    pub target: i128,
    pub chunk_size: usize,
}

pub enum PiCalcSignal {
    Start,
    Stop,
    Pause,
}

//function for running the hub connector
pub fn hub_runner(status_update_var: Arc<Mutex<PiCalcUpdate>>, singal: Receiver<PiCalcSignal>) {
    let listener = TcpListener::bind("127.0.0.1:13021").unwrap();

    //used to store the connections
    let mut connections: Vec<Connection> = vec![];

    listener.set_nonblocking(true).unwrap();

    let mut spoke_id_counter = 0;

    let mut calculation_thread: Option<thread::JoinHandle<()>> = None;

    let (task_tx, connection_rx) = unbounded();
    let (connection_tx, task_rx) = unbounded();

    //waits for connections
    loop {


        //dispatches tasks to workers
        if !connection_rx.is_empty() && !connections.is_empty(){
            for connection in &mut connections{
                if connection.tasks != connection.threads as usize{
                    connection.tasks += 1;
                    let task: TaskPass = connection_rx.try_recv().unwrap();
                    ciborium::into_writer(&task, &connection.socket).unwrap();
                    break;
                }
            }
        }


        for connection in &mut connections{
            if connection.tasks > 0{
                let mut buf = [10];
                if connection.socket.peek(&mut buf).unwrap() > 1{
                    let result = ciborium::from_reader(&connection.socket).unwrap();
                    let _ = connection_tx.send(result);
                    connection.tasks -= 1;
                }
            }
        }

        let some_key = match singal.try_recv() {
            Ok(key) => Some(key),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        };

        //starts the thread if need be
        if let Some(PiCalcSignal::Start) = some_key {
            //see if computation was already started
            if let Some(ref handle) = calculation_thread {
                if !handle.is_finished() {
                    println!("computation already started");
                    continue;
                }
            }
            println!("computation started");
            let runner_update = Arc::clone(&status_update_var);
            let chunk_size = runner_update.lock().unwrap().chunk_size;
            let task_rx: crossbeam_channel::Receiver<TaskPass> = task_rx.clone();
            let task_tx: crossbeam_channel::Sender<TaskPass> = task_tx.clone();
            calculation_thread = Some(thread::spawn(move || {
                println!("starting the pi calculations");

                runner_update.lock().unwrap().status = PiCalcStatus::Running;
                let now = SystemTime::now();
                let target = runner_update.lock().unwrap().target;
                let calculated_pi =
                    chudnovsky(target, &task_tx, &task_rx, chunk_size).to_string();
                let duration = now.elapsed().expect("unexpected time error");
                runner_update.lock().unwrap().status = PiCalcStatus::Stopped;

                let calculated_pi = calculated_pi
                    .char_indices()
                    .nth_back(22)
                    .map(|(i, _)| &calculated_pi[i..])
                    .unwrap()
                    .to_string();
                let mut calculated_pi = calculated_pi.chars();
                calculated_pi.next_back();
                calculated_pi.next_back();

                runner_update.lock().unwrap().last_20 =
                    Some(calculated_pi.as_str().to_string());
                runner_update.lock().unwrap().duration = Some(duration.as_secs_f64());
            }));
        }

        let (mut socket, _addr) = match listener.accept() {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => continue,
                _ => panic!("unkown error continuing "),
            },
        };

        let system_info: SystemInfo = ciborium::from_reader(&mut socket).unwrap();

        //verivys system information
        match system_info.api_version {
            0..API_VERSION => {
                println!("connection version too low");
                socket.shutdown(std::net::Shutdown::Write).unwrap();
                continue;
            }
            API_VERSION => (),
            _ => {
                println!("api version too high update the hub");
                socket.shutdown(std::net::Shutdown::Write).unwrap();
                continue;
            }
        }

        println!("spoke connected");
        connections.push(Connection {
            socket,
            threads: system_info.cores as u16,
            id: spoke_id_counter,
            tasks: 0
        });
        status_update_var.lock().unwrap().spokes.push(SpokeInfo {
            id: spoke_id_counter,
            cores: system_info.cores,
        });
        spoke_id_counter += 1;
    }
}
