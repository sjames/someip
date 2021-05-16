use std::io;

use generator_macros::*;

pub trait Hello {
    fn echo_int(&mut self,value:i32) -> Result<i32,io::Error>;
    fn echo_string(&mut self,value: String) -> Result<String,io::Error>;
}

#[service(
    fields(1 => value1:i32, 2=> value2:String, 3=> value3: u32),
    notifications(1 => value1:i32, 2=> value2:String, 3=> value3: u32),
    method_ids(0 => echo_int, 1 => echo_string, foo => 2, bar => 3 )
)]
pub trait HelloWorld {
    fn echo_int(&mut self,value:i32) -> Result<i32,io::Error>;
    fn echo_string(&mut self,value: String) -> Result<String,io::Error>;
}