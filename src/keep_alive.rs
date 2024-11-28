use std::net::TcpListener;
use std::io::Write;

pub fn keep_alive() {
   std::thread::spawn(|| {
      let listener = TcpListener::bind("0.0.0.0:8080").unwrap();

      for stream in listener.incoming() {
         if let Ok(mut stream) = stream {
            let res = "HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\nI'm alive";
            stream.write_all(res.as_bytes()).unwrap();
         }
      }
   });
}