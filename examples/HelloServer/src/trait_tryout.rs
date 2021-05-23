use serde::{Serialize, Deserialize};

mod interface {
    use generator_macros::*;
    use std::io;
    use serde::{Serialize, Deserialize};


    #[derive(Serialize,Deserialize)]
    pub struct Field1 {
        a: u32,
        b: u16,
        c: String,
    }

    #[derive(Serialize,Deserialize)]
    pub struct Event1 {
        a: u32,
        b: String,
        c: String,
    }

    #[service(fields([1]value1:Field1,[2]value2:String, [3]value3: u32),
        events([1 =>10]value1:Event1, [2=>10]value2:String, [3=>10]value3: u32), 
        method_ids([1]echo_int, [2]echo_string))]
    pub trait HelloWorld {
        fn echo_int(&self, value: i32, value1: Field1) -> Result<i32, io::Error>;
        fn echo_string(&self, value: String) -> Result<String, io::Error>;
    }
}

struct HelloWorldServer {

}

impl interface::HelloWorld for HelloWorldServer {
    fn echo_int(&self, value: i32, value1: interface::Field1) -> Result<i32, std::io::Error> {
        todo!()
    }

    fn echo_string(&self, value: String) -> Result<String, std::io::Error> {
        todo!()
    }

    fn set_value1(& self, _: interface::Field1) -> Result<(), std::io::Error> { todo!() }

    fn get_value1(&self) -> Result<&interface::Field1, std::io::Error> { todo!() }
    fn set_value2(&self, _: std::string::String) -> Result<(), std::io::Error> { todo!() }
    fn get_value2(&self) -> Result<&std::string::String, std::io::Error> { todo!() }
    fn set_value3(&self, _: u32) -> Result<(), std::io::Error> { todo!() }
    fn get_value3(&self) -> Result<&u32, std::io::Error> { todo!() }

}