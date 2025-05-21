
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::{sync::mpsc, thread};
use std::net::TcpStream;
use std::io::ErrorKind;
use std::env;

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
    Compute((Integer, Integer, Integer), (Integer, Integer, Integer), (i128, i128)),
    Result((Integer, Integer, Integer), (i128, i128)), //finilazation chunk of data
    AWK,
}


#[derive(Serialize)]
struct SystemInfo{
    api_version: usize,
    cores: usize,
}

//compute handeler
fn computation_handeler(task: TaskPass, threads: &mut Vec<thread::JoinHandle<()>>, return_channels: &mut Vec<Receiver<TaskPass>>){
    match task{
        TaskPass::Compute((pam, qam, ram), (pmb, qmb, rmb), range) => {
            let (tx,rx) = mpsc::channel();
            return_channels.push(rx);
            threads.push(thread::spawn(move || {
                println!("got task with range: {:?}", range);
                let pab = &pam * pmb;
                let qab = qam * &qmb;
                let rab = qmb * ram + pam * rmb;
                tx.send(TaskPass::Result((pab, qab, rab), range)).unwrap();
            }));
        },
        TaskPass::Range(begin, end) => {
            let (tx,rx) = mpsc::channel();
            return_channels.push(rx);
            threads.push(thread::spawn(move || {
                println!("got task with range: {:?}", (begin, end));
                let result = pi_calc::bin_split(begin, end);
                tx.send(TaskPass::Result(result, (begin, end))).unwrap();
            }));
        },
        TaskPass::Result(_,_) => panic!("hub shuld never send result"),
        TaskPass::AWK => (),
    }
}

fn main() {
    let args: Vec<(String,String)> = env::vars().collect();
    let hub_url = args.iter().find(|(key, _)| key == "HUB_URL").expect("HUB_URL is not set");
    let threads: usize = args.iter().find(|(key, _)| key == "THREADS").expect("THREADS is not set").1.parse().expect("THREADS is not an interger");
    let system_info = SystemInfo{api_version: API_VERSION, cores: threads};
    let mut threads =vec![];
    let mut return_channels = vec![];
    loop {
        let stream = loop{
            println!("attempting a connection at : {}",hub_url.1.clone());
            match TcpStream::connect(hub_url.1.clone()){
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

        let mut task_count =0;
        loop{
            //accept signals
            let mut buf = [1;100];
            stream.set_nonblocking(true).unwrap();
            let recive_length = stream.peek(&mut buf).unwrap_or(0);
            stream.set_nonblocking(false).unwrap();

            if recive_length > 2{
            
                let maybe_task =  ciborium::from_reader(&stream);
                match maybe_task {
                    Ok(task) => {
                        task_count += 1;
                        computation_handeler(task, &mut threads, &mut return_channels);
                        ciborium::into_writer(&TaskPass::AWK, &stream).unwrap();
                    }
                    Err(e) => {
                        match e{
                            ciborium::de::Error::Io(e) => {
                                match e.kind() {
                                    std::io::ErrorKind::WouldBlock => (),
                                   error => {
                                        println!("recive error : {}", error);
                                        break;
                                    }
                                }
                            },
                            error => {
                                println!("recive error : {}", error);
                                break;
                            }
                        }
                    }
                }
            }

            return_channels.retain(|channel| {
                match channel.try_recv() {
                    Ok(value) => {
                        //block until we can continue
                        if let TaskPass::Result(_, ( a , b)) = value { println!("retrunting a range of {} to {}", a , b) }
                        ciborium::into_writer(&value, &stream).unwrap(); // Handle errors as needed
                        task_count -= 1;
                        false
                    }
                    Err(_) => true
                }
            });
            thread::sleep(Duration::from_millis(10));
        }
        let _  = stream.shutdown(std::net::Shutdown::Both);
        println!("scoket disconected");
    }
}
