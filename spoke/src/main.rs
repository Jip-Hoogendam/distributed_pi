
use std::sync::mpsc::Receiver;
use std::thread::Thread;
use std::{sync::mpsc, thread};
use std::net::TcpStream;
use std::io::{Error, ErrorKind};

use rug::Integer;
use serde::{Deserialize,Serialize};

//for possible compatibility issues
const API_VERSION: usize = 4;


//pi calculation based on the wikipedia artivle on the chudnovsky algorithem
mod pi_calc{
    use rug::Integer;

    //an binay split algorithem
    pub fn bin_split(a: i128, b:i128) -> (rug::Integer,rug::Integer,rug::Integer){
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
}

//used to send the tasks for the spokes
#[derive(Serialize, Deserialize, Debug)]
enum TaskPass {
    Range(i128, i128), //a range of points to calculate
    Compute((Integer, Integer, Integer), (Integer, Integer, Integer)),
    Result((Integer, Integer, Integer)), //finilazation chunk of data
}



#[derive(Serialize)]
struct SystemInfo{
    api_version: usize,
    cores: usize,
}

//compute handeler
fn computation_handeler(task: TaskPass, threads: &mut Vec<thread::JoinHandle<()>>, return_channels: &mut Vec<Receiver<TaskPass>>){
    println!("got tasks : {:?}", task);
    match task{
        TaskPass::Compute((pam, qam, ram), (pmb, qmb, rmb)) => {
            let (tx,rx) = mpsc::channel();
            return_channels.push(rx);
            threads.push(thread::spawn(move || {
                let pab = &pam * pmb;
                let qab = qam * &qmb;
                let rab = qmb * ram + pam * rmb;
                tx.send(TaskPass::Result((pab, qab, rab))).unwrap();
            }));
        },
        TaskPass::Range(begin, end) => {
            let (tx,rx) = mpsc::channel();
            return_channels.push(rx);
            threads.push(thread::spawn(move || {
                let result = pi_calc::bin_split(begin, end);
                tx.send(TaskPass::Result(result)).unwrap();
            }));
        },
        TaskPass::Result(_) => panic!("cant compute on result")
    }
}

fn main() {
    let system_info = SystemInfo{api_version: API_VERSION, cores: 16};
    let mut threads =vec![];
    let mut return_channels = vec![];
    loop {
        let stream = loop{
            match TcpStream::connect("127.0.0.1:13021"){
                Ok(stream) => break stream,
                Err(e) => {
                    match e.kind(){
                        ErrorKind::NetworkDown => panic!("network is down"),
                        ErrorKind::ConnectionRefused => {
                            println!("connection failed. Retrying in 5 seconds");

                            let five_seconds = std::time::Duration::from_millis(5000);
                            thread::sleep(five_seconds);
                            continue;
                        },
                        _ => panic!("unkown error : \n{}", e)
                    }
                } 
            }
            
        };

        println!("connected to hub");
        
        match ciborium::into_writer(&system_info, &stream){
            Ok(value) => value,
            Err(_) =>{
                println!("seems like the connection messed up restting the connection");
                let _ =stream.shutdown(std::net::Shutdown::Both);
                continue;
            }
        };
        
        stream.set_nonblocking(true).unwrap();
        loop{   
            //accept signals
            let maybe_task =  ciborium::from_reader(&stream);
            match maybe_task {
                Ok(task) => {
                    computation_handeler(task, &mut threads, &mut return_channels);
                }
                Err(e) => {
                    match e{
                        ciborium::de::Error::Io(e) => {
                            match e.kind() {
                                std::io::ErrorKind::WouldBlock => (),
                                 _ => {
                                    let _  = stream.shutdown(std::net::Shutdown::Both);
                                    println!("scoket disconected");
                                    break;
                                 }
                            }
                        },
                        error => {
                            println!("{}", error);
                            let _  = stream.shutdown(std::net::Shutdown::Both);
                            println!("scoket disconected");
                            break;
                        }
                    }
                }
            }

            return_channels.retain(|channel| {
                match channel.try_recv() {
                    Ok(value) => {
                        //block until we can continue
                        stream.set_nonblocking(false).unwrap();
                        ciborium::into_writer(&value, &stream).unwrap(); // Handle errors as needed
                        stream.set_nonblocking(true).unwrap();
                        println!("returned data");
                        false
                    }
                    Err(_) => true
                }
            });
        }
    }
}
