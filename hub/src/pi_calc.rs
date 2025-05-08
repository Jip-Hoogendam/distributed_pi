//module for the calculation of pi


use core::f64;

use rug::{Float, Integer, Rational};
use serde::{Serialize,Deserialize};
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, SystemTime};
use std::fs;
use std::{i64, io::{stdout, Write}, net::{TcpListener, TcpStream}, thread::sleep};

#[derive(Serialize, Deserialize, Debug)]
struct ComputeResult{
    result: (Integer, Integer, Integer)
}


use std::io;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use std::thread;

//configurations
const TARGET_DIGETS:i128 = 100_000_000;

//for possible compatibility issues
const API_VERSION:usize = 3;

//used to send the tasks for the spokes
#[derive(Serialize, Deserialize, Debug)]
enum TaskPass {
    Data(Vec<(i128, i128)>),
}

struct Connection{
    socket: TcpStream,
    threads: u16,
    id: usize,
}

#[derive(Deserialize)]
struct SystemInfo{
    api_version: usize,
    cores: usize,
}


//an binay split algorithem
fn bin_split(a: i128, b:i128) -> (rug::Integer,rug::Integer,rug::Integer){
    if b == a + 1{

    let pab = Integer::from(-(6*a - 5)*(2*a - 1)*(6*a - 1));
    let qab = Integer::from(10939058860032000 * a.pow(3));
    let rab = Integer::from(&pab * (545140134*a + 13591409));
    (pab, qab, rab)
    }else{
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
struct BinaryReconstruction{
    input: (i128,i128),
}

fn fast_bin_split(n: i128, connections: &mut Vec<Connection>)  -> (rug::Integer,rug::Integer,rug::Integer){

    let mut thread_count = 0;
    for connection in &mut *connections{
        thread_count += connection.threads as usize;
    }


    //makes shure the tasks can be sent out
    let mut iterations = n;
    loop {
        iterations += iterations % 2;
        iterations += iterations % thread_count as i128;
        if iterations % 2 == 0{
            break;
        }
    }
    
    let chunks = iterations as usize / thread_count;


    //splits the cases with the desired lengths.
    let mut reconstruction_array: Vec<BinaryReconstruction> = vec![];
    

    for i in 0..thread_count{
        if i == 0 {
            reconstruction_array.push(BinaryReconstruction { input: ((i * chunks) as i128 + 1 , ((i + 1) * chunks) as i128) });
        }else{
            reconstruction_array.push(BinaryReconstruction { input: ((i * chunks) as i128 , ((i + 1) * chunks) as i128) });
        }
    }

    let target_threads = (reconstruction_array.len() as f32/connections.len()as f32).ceil() as usize;

    for connection in &mut *connections {
        let mut task = vec![];
        for _ in 0..target_threads{
            let value = match reconstruction_array.pop(){
                Some(value) => value,
                None => break,
            };
            task.push(value.input);
        }

        let _ = ciborium::into_writer(&TaskPass::Data(task), &connection.socket);
    }
    


    let mut reconstruction_array: Vec<(Integer, Integer, Integer)> = vec![];

    
    //recives the computations from spokes
    for connection in connections{
        println!("waiting for data");
        let responce: ComputeResult = ciborium::from_reader(&mut connection.socket).unwrap();
        println!("recived data");
        
        
        reconstruction_array.push(responce.result);
    }
    
    for reconstrucion in &reconstruction_array{
        if reconstrucion.0  == 0{
            println!(" zero found");
        }
        if reconstrucion.1  == 0{
            println!(" zero found");
        }
        if reconstrucion.2  == 0{
            println!(" zero found");
        }
    }
    println!("recived all data finilizing computing");

    //continuasly rebuilds the threads given untill there is one left.
    while reconstruction_array.len() > 1{
        let length =reconstruction_array.len();
        for _ in 0..length/2{
            let (pam, qam, ram) = reconstruction_array.pop().unwrap();
            let (pmb, qmb, rmb) = reconstruction_array.pop().unwrap();
            let pab = &pam * pmb;
            let qab = qam * &qmb;
            let rab = qmb * ram + pam * rmb;
            reconstruction_array.push((pab, qab, rab));
        }
    }
    reconstruction_array.pop().unwrap()
}

//runs the chudnovsky algorithem where n is the percision number
fn chudnovsky(n: i128, connections: &mut Vec<Connection>) -> rug::Float{
    let (_p1n, q1n, r1n) = fast_bin_split(n / 14, connections);
    let bits = (n as f64 * f64::consts::LOG2_10 + 16.0).ceil() as u32;

    let c      = Float::with_val(bits, 426880);
    let sqrt05 = Float::with_val(bits, 10005).sqrt();

    // build the rational (13591409·q + r) / q
    let denom_rat = Rational::from((13591409 * q1n.clone() + r1n, q1n));
    let denom_f   = Float::with_val(bits, denom_rat);

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
pub struct SpokeInfo{
    id: usize,
    cores: usize,
}

#[derive(Serialize, Clone)]
pub struct  PiCalcUpdate {
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
pub fn hub_runner(status_update_var: Arc<Mutex<PiCalcUpdate>>, singal: Receiver<PiCalcSignal>){
    let listener = TcpListener::bind("127.0.0.1:13021").unwrap();

    //used to store the connections
    let mut connections: Vec<Connection> = vec![];


    listener.set_nonblocking(true).unwrap();

    let mut spoke_id_counter = 0;

    let mut calculation_thread;

    //waits for connections
    loop{
        sleep(std::time::Duration::from_millis(100));

        let some_key = match singal.try_recv() {
            Ok(key) => Some(key),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        };

        if let Some(PiCalcSignal::Start) = some_key{
            println!("computation started");
            let runner_update = Arc::clone(&status_update_var);
            calculation_thread = thread::spawn(move || {
                println!("starting the pi calculations");
    
                runner_update.lock().unwrap().status = PiCalcStatus::Running;
                let now = SystemTime::now();
                let target = runner_update.lock().unwrap().target;
                let calculated_pi = chudnovsky(target, &mut connections).to_string();
                let duration = now.elapsed().expect("unexpected time error");
                runner_update.lock().unwrap().status = PiCalcStatus::Stopped;

                let calculated_pi = calculated_pi.char_indices().nth_back(22).map(|(i, _)| &calculated_pi[i..]).unwrap().to_string();
                let mut calculated_pi = calculated_pi.chars();
                calculated_pi.next_back();
                calculated_pi.next_back();


                runner_update.lock().unwrap().last_20 = Some(calculated_pi.as_str().to_string());
                runner_update.lock().unwrap().duration = Some(duration.as_secs_f64());
            });
        }


        let (mut socket, _addr) = match listener.accept() {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::WouldBlock => continue,
                    _ => panic!("unkown error continuing "),
                }
            },
        };

        let system_info: SystemInfo = ciborium::from_reader(&mut socket).unwrap();

        //verivys system information
        match system_info.api_version{
            0..API_VERSION => {
                println!("connection version too low");
                socket.shutdown(std::net::Shutdown::Write).unwrap();
                continue;
            },
            API_VERSION => (),
            _ => {
                println!("api version too high update the hub");
                socket.shutdown(std::net::Shutdown::Write).unwrap();
                continue;
            }
        }

        println!("spoke connected");
        connections.push(Connection{socket, threads: system_info.cores as u16, id: spoke_id_counter});
        status_update_var.lock().unwrap().spokes.push(SpokeInfo { id: spoke_id_counter, cores: system_info.cores });
        spoke_id_counter += 1;
    }
}
