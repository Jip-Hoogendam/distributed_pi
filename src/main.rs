


//pi calculation based on the wikipedia artivle on the chudnovsky algorithem
mod pi_calc{
    pub static DEFINED_PERCICION : u32 = 1000000;

    use rug::{Float};

    //an binay split algorithem
    fn bin_split(a: i128, b:i128) -> (rug::Float,rug::Float,rug::Float){
        if b == a + 1{
           let Pab = Float::with_val(DEFINED_PERCICION,-(6*a - 5)*(2*a - 1)*(6*a - 1));
           let Qab = Float::with_val(DEFINED_PERCICION,10939058860032000 * a.pow(3));
           let Rab = Float::with_val(DEFINED_PERCICION,&Pab * (545140134*a + 13591409));
           return  (Pab, Qab, Rab);
        }else{
            let m = (a + b) / 2; // 2
            let (Pam, Qam, Ram) = bin_split(a, m);
            let (Pmb, Qmb, Rmb) = bin_split(m, b);
            
            let Pab = &Pam * Pmb;
            let Qab = Qam * &Qmb;
            let Rab = Qmb * Ram + Pam * Rmb;
            return  (Pab, Qab, Rab);
        }
    }


    //runs the chudnovsky algorithem where n is the percision number
    pub fn chudnovsky(n: i128) -> rug::Float{
        let (_P1n, Q1n, R1n) = bin_split(1, n);
        (426880.0 * Float::with_val(DEFINED_PERCICION, 10005).sqrt() * &Q1n) / (13591409*Q1n + R1n)
    }
}

fn main() {
    println!("Hello, world! pi ={}", pi_calc::chudnovsky(100000));
}
