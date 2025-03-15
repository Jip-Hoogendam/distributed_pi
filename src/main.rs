
use std::time::SystemTime;
use std::fs;

//pi calculation based on the wikipedia artivle on the chudnovsky algorithem
mod pi_calc{
    static DEFINED_PERCICION : u32 = 1000000;

    use std::{collections::VecDeque, sync::mpsc, thread};

    use rug::{Float, Integer};

    //an binay split algorithem
    fn bin_split(a: i128, b:i128) -> (rug::Integer,rug::Integer,rug::Integer){
        if b == a + 1{

           let pab = Integer::from(-(6*a - 5)*(2*a - 1)*(6*a - 1));
           let qab = Integer::from(10939058860032000 * a.pow(3));
           let rab = Integer::from(&pab * (545140134*a + 13591409));
           return  (pab, qab, rab);
        }else{
            let m = (a + b) / 2;
            let (pam, qam, ram) = bin_split(a, m);
            let (pmb, qmb, rmb) = bin_split(m, b);
            
            let pab = &pam * pmb;
            let qab = qam * &qmb;
            let rab = qmb * ram + pam * rmb;
            return  (pab, qab, rab);
        }
    }

    //used to store the result and the ranges that it must compute with.
    struct BinaryReconstruction{
        input: (i128,i128),
        value: (rug::Integer,rug::Integer,rug::Integer)
    }

    //less memory efficiant version.that is likely faster
    fn fast_bin_split(n: i128, thread_count: u8) -> (rug::Integer,rug::Integer,rug::Integer){

        //stores the depht at witch the depth is enough to fit the amout of threads
        let mut depth: u8 = 0;
        loop {
            if depth.pow(2) > thread_count{
                depth -= 1;
                break;
            }
            depth += 1;
        }


        //the root of the reconstructed binary split
        let root_reconstruction = BinaryReconstruction{input: (1,n),value: (Integer::from(0),Integer::from(0),Integer::from(0))};

        //splits the cases with the desired lengths.
        let mut reconstruction_array: VecDeque<BinaryReconstruction> = vec![].into();
        reconstruction_array.push_back(root_reconstruction);
        for _ in 0..depth{
            let length =reconstruction_array.len();
            for _ in 0..length{
                let bin_split = reconstruction_array.pop_front().unwrap();
                let a = bin_split.input.0;
                let b = bin_split.input.1;

                //cant solve it if the number of approximations is too small.
                if b == a + 1{
                    panic!("n is set too low for the fast bin split algorithem");
                }

                let m = (a + b) / 2;
                let bin_reconstruction_1 = BinaryReconstruction{input: (a,m),value: (Integer::from(0),Integer::from(0),Integer::from(0))};
                reconstruction_array.push_back(bin_reconstruction_1);
                let bin_reconstruction_2 = BinaryReconstruction{input: (m,b),value: (Integer::from(0),Integer::from(0),Integer::from(0))};
                reconstruction_array.push_back(bin_reconstruction_2);
            }
        }

        let mut threads = vec![];
        let mut return_channels = vec![];


        //spawns the threads to search the binay ranges given.
        for mut bin_recon in reconstruction_array{
            let (tx,rx) = mpsc::channel();
            return_channels.push(rx);
            threads.push(thread::spawn(move || {
                bin_recon.value = bin_split(bin_recon.input.0, bin_recon.input.1);
                tx.send(bin_recon.value).unwrap();
            }));
        }
        

        for thread in threads{
            thread.join().unwrap();
        }

        let mut reconstruction_array: VecDeque<(Integer, Integer, Integer)> = vec![].into();

        for channel in return_channels{
            reconstruction_array.push_back(channel.recv().unwrap());
        }


        //continuasly rebuilds the threads given untill there is one left.
        while reconstruction_array.len() > 1{
            let length =reconstruction_array.len();
            for _ in 0..length/2{
                let (pam, qam, ram) = reconstruction_array.pop_front().unwrap();
                let (pmb, qmb, rmb) = reconstruction_array.pop_front().unwrap();
                let pab = &pam * pmb;
                let qab = qam * &qmb;
                let rab = qmb * ram + pam * rmb;
                reconstruction_array.push_back((pab, qab, rab));
            }
        }
        return reconstruction_array.get(0).unwrap().clone();
    }

    //runs the chudnovsky algorithem where n is the percision number
    pub fn chudnovsky(n: i128) -> rug::Float{
        let (_p1n, q1n, r1n) = fast_bin_split(n, 16);
        (426880.0 * Float::with_val(DEFINED_PERCICION, 10005).sqrt() * &q1n) / (13591409*q1n + r1n)
    }
}



fn main() {

    //reads in a verification file to see if it correct
    println!("reading veri_pi(thihi).txt");

    let contents: Vec<char> = fs::read_to_string("./veri_pi(thihi).txt")
        .expect("Should have been able to read the file").chars().collect();

    let now = SystemTime::now();
    println!("starting the pi calculations");
    let calculated_pi: Vec<char> = pi_calc::chudnovsky(10000000).to_string().chars().collect();


    let elapsed_seconds = match now.elapsed() {
        Ok(value) => value.as_secs_f64(),
        Err(_) => panic!("system time error"),
    };
    println!("calculated pi in {} seconds", elapsed_seconds);    

    
    for (index,number) in calculated_pi.into_iter().enumerate(){
        if number == contents[index]{
            print!("\x1b[92m{}",number);
        }else{
            print!("\x1b[91m{}",number);
        }
    }
    println!("\x1b[0m");
}
