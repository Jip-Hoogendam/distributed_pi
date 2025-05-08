
use std::{sync::mpsc, thread};
use std::net::TcpStream;
use std::io::{Error, ErrorKind};

use rug::Integer;
use serde::{Deserialize,Serialize};

//for possible compatibility issues
const API_VERSION: usize = 3;


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
    Data(Vec<(i128, i128)>),
}
#[derive(Serialize, Deserialize, Debug)]
struct ComputeResult{
    result: (Integer, Integer, Integer)
}


#[derive(Serialize)]
struct SystemInfo{
    api_version: usize,
    cores: usize,
}

//compute handeler
fn computation_handeler(stream: &mut TcpStream) -> Result<(), Error>{
    let deserilized_value: TaskPass = ciborium::from_reader(&*stream).unwrap();
    println!("got tasks : {:?}", deserilized_value);
    let TaskPass::Data(result_value) = deserilized_value;
    

    //starts the threads on the given data
    let mut threads = vec![];
    let mut return_channels = vec![];


    //spawns the threads to search the binay ranges given.
    for bin_recon in result_value{
        let (tx,rx) = mpsc::channel();
        return_channels.push(rx);
        threads.push(thread::spawn(move || {
            let result = pi_calc::bin_split(bin_recon.0, bin_recon.1);
            tx.send(result).unwrap();
        }));
    }
    

    for thread in threads{
        thread.join().unwrap();
    }

    let mut results = vec![];

    for channel in return_channels{
        let (pab, qab, rab) = channel.recv().unwrap();
        results.push((pab, qab, rab));
    }

    //continuasly rebuilds the threads given untill there is one left.
    while results.len() > 1{
        let length =results.len();
        for _ in 0..length/2{
            let (pam, qam, ram) = results.pop().unwrap();
            let (pmb, qmb, rmb) = results.pop().unwrap();
            let pab = &pam * pmb;
            let qab = qam * &qmb;
            let rab = qmb * ram + pam * rmb;
            results.push((pab, qab, rab));
        }
    }


    let _ = ciborium::into_writer(&ComputeResult{result: results.pop().unwrap()}, stream);
    
    println!("my job is done");
    Ok(())
}

fn main() {
    let system_info = SystemInfo{api_version: API_VERSION, cores: 16};

    loop {
        let mut stream = loop{
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

        loop{
            match computation_handeler(&mut stream){
                Ok(value) => value,
                Err(_) =>{
                    println!("seems like the connection messed up restting the connection");
                    let _ =stream.shutdown(std::net::Shutdown::Both);
                    break;
                }
            }
        }
    }
}
