use std::{io::{stdout, Write}, net::{TcpListener, TcpStream}, thread::sleep};
use serde::{Deserialize,Serialize};
use std::time::SystemTime;
use std::fs;

use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::thread;

//for possible compatibility issues
const API_VERSION:usize = 3;


//used to send the tasks for the spokes
#[derive(Serialize, Deserialize, Debug)]
enum TaskPass {
    Data(Vec<(i128, i128)>),
}

#[derive(Serialize, Deserialize, Debug)]
struct ComputeResult{
    result: Vec<(String, String, String)>
}



struct Connection{
    socket: TcpStream,
    threads: u16,
}


//used to wait for the enter press without blocking gotten from : https://stackoverflow.com/questions/30012995/how-can-i-read-non-blocking-from-stdin
fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}

#[derive(Deserialize)]
struct SystemInfo{
    api_version: usize,
    cores: usize,
}

//pi calculation based on the wikipedia artivle on the chudnovsky algorithem
mod pi_calc{


    use rug::{Float, Integer};
    use serde::{Serialize,Deserialize};

    use crate::{Connection, TaskPass};

    #[derive(Serialize, Deserialize, Debug)]
    struct ComputeResult{
        result: (Integer, Integer, Integer)
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

        let n = iterations;
        let chunks = n as usize / thread_count;


        //splits the cases with the desired lengths.
        let mut reconstruction_array: Vec<BinaryReconstruction> = vec![];
        
        for i in 0..thread_count{
            reconstruction_array.push(BinaryReconstruction { input: ((i * chunks + 1) as i128, ((i + 1) * chunks + 1) as i128) });
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
    pub fn chudnovsky(n: i128, connections: &mut Vec<Connection>) -> rug::Float{
        let (_p1n, q1n, r1n) = fast_bin_split(n, connections);
        println!("last step before victory");
        (426880.0 * Float::with_val((n* 100) as u32, 10005).sqrt() * &q1n) / (13591409*q1n + r1n)
    }
}


fn main() {
    let listener = TcpListener::bind("127.0.0.1:13021").unwrap();

    //used to store the connections
    let mut connections = vec![];

    println!("press enter to begin calculating pi");
    stdout().flush().unwrap();

    listener.set_nonblocking(true).unwrap();

    let stdin_channel = spawn_stdin_channel();

    //waits for connections
    loop{
        sleep(std::time::Duration::from_millis(100));

        let some_key = match stdin_channel.try_recv() {
            Ok(key) => Some(key),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        };

        if some_key.is_some() {
            break;
        }


        let (mut socket, addr) = match listener.accept() {
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

        connections.push(Connection{socket, threads: system_info.cores as u16});
    
        println!("new host at {}", addr);
    }

    //reads in a verification file to see if it correct
    println!("reading veri_pi(thihi).txt");

    let contents: Vec<char> = fs::read_to_string("../veri_pi(thihi).txt")
        .expect("Should have been able to read the file").chars().collect();

    let now = SystemTime::now();
    println!("starting the pi calculations");


    let calculated_pi: Vec<char> = pi_calc::chudnovsky(10_000_000, &mut connections).to_string().chars().collect();

    let elapsed_seconds = match now.elapsed() {
        Ok(value) => value.as_secs_f64(),
        Err(_) => panic!("system time error"),
    };

    let mut result_index = 0;
    for (index,number) in calculated_pi.into_iter().enumerate(){
        match contents.get(index) {
            Some(char) => {
                if number == *char{
                }else{
                    result_index = index;
                    break;
                }
            }
            None => {
                result_index = index;
                break;
            }
        }; 
        
    }
    println!("\x1b[0m");

    println!("calculated {} digets of pi in {} seconds before an incorrect diget was found", result_index, elapsed_seconds);
}
