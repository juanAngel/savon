use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    savon::gen::gen_write("./countrinfoservice.wsdl", &out_dir).unwrap();
}
