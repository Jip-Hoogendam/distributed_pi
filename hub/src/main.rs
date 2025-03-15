use std::{io::{stdin,stdout,Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, thread::sleep};
use serde::{Deserialize,Serialize};


use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::thread;

//used to send the tasks for the spokes
#[derive(Serialize, Deserialize, Debug)]
struct TaskPass{
    start: i128,
    end: i128,
}

#[derive(Serialize, Deserialize, Debug)]
struct ComputeResult{
    data: String
}

struct connection{
    socket: TcpStream,
    addr: SocketAddr,
    task: TaskPass,
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


fn main() {
    let listener = TcpListener::bind("127.0.0.1:13021").unwrap();

    //used to store the connections
    let mut connections = vec![];

    let mut begin_a = 1;
    let mut begin_b = 2;

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


        let (socket, addr) = match listener.accept() {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::WouldBlock => continue,
                    _ => panic!("unkown error continuing "),
                }
            },
        };

        connections.push(connection{socket, addr, task: TaskPass{start: begin_a, end: begin_b}});

        begin_a = begin_b;
        begin_b += 1;
    
        println!("new host at {}", addr);
    }

    for connection in &mut connections{
        let _ = connection.socket.write( &rmp_serde::to_vec(&TaskPass{start: connection.task.start, end: connection.task.end}).unwrap());
    }

    // let _ = socket.write( &rmp_serde::to_vec(&taskPass{start: 1, end: 2}).unwrap());
    // let responce: &mut [u8] = &mut [];
    // match socket.read(responce) {
    //     Ok(amount) => println!("connection responded with : {} bytes", amount),
    //     Err(_) => println!("conection never responded :(")
    // }; 

    for connection in &mut connections{
        let responce: &mut [u8] = &mut [0;128];
        let _ = connection.socket.read(responce).unwrap();

        let responce: ComputeResult = rmp_serde::from_read(&*responce).unwrap();

        println!("spoke responded with : {:?}", responce);
    }


    for connection in connections{
        let _ = connection.socket.shutdown(std::net::Shutdown::Both);
    }
    // let _ = socket.shutdown(std::net::Shutdown::Both);
}
