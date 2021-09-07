# someip

This is an implementation of the [SOME/IP](https://some-ip.com/) protocol, with some caveats.

The serialization is bincode, so it will not interact with other implementations of SOME/IP.  
This should be easy to solve as the SOME/IP serialization is quite simple.

Interfaces are defined by using traits and a derive macro. Look in the tests.rs file for a simple example.

Here is how you would define a service.

```rust
    #[service(
        name("org.hello.service"),
        fields([1]value1:Field1,[2]value2:String, [3]value3: u32),
        events([1 ;10]value1:Event1, [2;10]value2:String, [3;10]value3: u32), 
        method_ids([2]echo_string, [3]no_reply),
        method_ids([5]echo_struct)
    )]
    #[async_trait]
    pub trait EchoServer {
        fn echo_int(&self, value: i32) -> Result<i32, EchoError>;
        async fn echo_string(&self, value: String) -> Result<String, EchoError>;
        fn no_reply(&self, value: Field1);
        fn echo_u64(&self, value: u64) -> Result<u64, EchoError>;
        fn echo_struct(&self, value : Field1) -> Result<Field1, EchoError>;
    }
```

Though very early in its implementation, I decided to make it public. 


