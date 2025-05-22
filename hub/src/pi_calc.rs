//module for the calculation of pi

use core::f64;
use std::fs::File;
use std::io::Write;
use rug::{Float, Integer, Rational};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, SystemTime};

#[derive(Serialize, Deserialize, Debug)]
struct ComputeResult {
    result: (Integer, Integer, Integer),
}

use crossbeam_channel::unbounded;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use std::thread;

//for possible compatibility issues
const API_VERSION: usize = 4;

//used to send the tasks for the spokes
#[derive(Serialize, Deserialize, Debug)]
enum TaskPass {
    Range(i128, i128), //a range of points to calculate
    Compute(
        (Integer, Integer, Integer),
        (Integer, Integer, Integer),
        (i128, i128),
    ),
    Result((Integer, Integer, Integer), (i128, i128)), //finilazation chunk of data
    AWK,
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
    let chunks = iterations as usize / chunksize ;

    let mut dispatched_tasks = 0;
    for i in 0..chunks {
        dispatched_tasks += 1;
        if i == 0 {
            task_dispatch
                .send(TaskPass::Range(
                    (i * chunksize) as i128 + 1,
                    ((i + 1) * chunksize) as i128,
                )).unwrap();
        } else {
            task_dispatch
                .send(TaskPass::Range(
                    (i * chunksize) as i128,
                    ((i + 1) * chunksize) as i128,
                )).unwrap();
        }
    }

    //dispatches the computation tasks to the connections
    let mut compute_results: Vec<TaskPass> = vec![];
    loop {
        if !task_return.is_empty() {
            if let TaskPass::Result(recived, (range_begin, range_end)) = task_return.recv().unwrap()
            {
                dispatched_tasks -= 1;
                println!("got result of {} to {} dispatched tasks {}", range_begin, range_end, dispatched_tasks);
                if let Some(pos) = &compute_results.iter().position(|x| match x {
                    TaskPass::Result(compare, (cmp_begin, cmp_end)) => {
                        if *cmp_end == range_begin {
                            let _ = task_dispatch.send(TaskPass::Compute(
                                compare.clone(),
                                recived.clone(),
                                (*cmp_begin, range_end),
                            ));
                            dispatched_tasks += 1;
                            return true;
                        }else if range_end == * cmp_begin{
                            let _ = task_dispatch.send(TaskPass::Compute(
                                recived.clone(),
                                compare.clone(),
                                (range_begin, *cmp_end),
                            ));
                            dispatched_tasks += 1;
                            return true;
                        }
                        false
                    }
                    _ => false,
                }) {
                    compute_results.swap_remove(*pos);
                } else {
                    compute_results.push(TaskPass::Result(recived, (range_begin, range_end)));
                }
            }else{
                println!("unkown error ?");
            }
        }
        if dispatched_tasks <= 0{
            break;
        }
    }

    match &compute_results[0] {
        TaskPass::Result(value, _) => value.clone(),
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
    let (_p1n, q1n,r1n) = fast_bin_split(n/13, task_dispatch, task_return, chunk_size);

    let bits = (n as f64 * f64::consts::LOG2_10 + 16.0).ceil() as u32;


    let c = Float::with_val(bits, 10005).sqrt() * 426880;

    c  / unsafe { Rational::from_canonical(13591409 * &q1n + r1n, q1n) }
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
    let listener = TcpListener::bind("0.0.0.0:13021").unwrap();

    listener.set_nonblocking(true).unwrap();

    let mut spoke_id_counter = 0;

    let mut calculation_thread: Option<thread::JoinHandle<()>> = None;

    let (task_tx, connection_rx) = unbounded();
    let (connection_tx, task_rx) = unbounded();

    //waits for connections
    loop {
        thread::sleep(Duration::from_millis(10));

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
                let calculated_pi = chudnovsky(target, &task_tx, &task_rx, chunk_size).to_string();
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

                runner_update.lock().unwrap().last_20 = Some(calculated_pi.as_str().to_string());
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

        let thread_count = system_info.cores;

        let connection_rx = connection_rx.clone();
        let connection_tx = connection_tx.clone();

        thread::spawn(move || {
            let mut sending = false;
            let mut tasks = 0;
            loop{
                //dispatches tasks to workers
                if !connection_rx.is_empty(){
                    let mut buf = [1; 60000];
                    socket.set_nonblocking(true).unwrap();
                    let recive_length = socket.peek(&mut buf).unwrap_or(0);
                    socket.set_nonblocking(false).unwrap();
                    if tasks != thread_count as usize && !sending && recive_length < 60000 {
                        let task: TaskPass = match connection_rx.try_recv() {
                            Ok(value) => value,
                            Err(crossbeam_channel::TryRecvError::Empty) => continue,
                            Err(_) =>panic!("channel unexpectedly disconnected"),
                        };
                        tasks += 1;

                        ciborium::into_writer(&task, &socket).unwrap();
                        sending = true;
                    }
                }

                //checks if something is recived from a socket
                let mut buf = [1; 100];
                socket.set_nonblocking(true).unwrap();
                let recive_length = socket.peek(&mut buf).unwrap_or(0);
                socket.set_nonblocking(false).unwrap();
                if recive_length > 1 {
                    if let Ok(result) = ciborium::from_reader(&socket) {
                        match result {
                            TaskPass::AWK => {
                                sending = false;
                            }
                            _ => {
                                let _ = connection_tx.send(result);
                                tasks -= 1;
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });

        status_update_var.lock().unwrap().spokes.push(SpokeInfo {
            id: spoke_id_counter,
            cores: system_info.cores,
        });
        spoke_id_counter += 1;
    }
}
