#![allow(dead_code, unused)]
use std::fs;
use std::io::BufReader;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use chrono::format::format;
use urlencoding::decode;
use chrono::{Datelike, Timelike, Utc};
use std::time::Duration;
use std::thread;
use notes_server::ThreadPool;

const MY_IP: &str = "192.168.1.3:7878";
const NOTESPATH: &str = "notes/notes.txt";

fn main() {
    let listener = TcpListener::bind("192.168.1.3:7878").unwrap();
    for stream in listener.incoming(){
        let stream= stream.unwrap();
        let pool = ThreadPool::new(4);
        
        println!("Connection established with {}", stream.peer_addr().unwrap());

        pool.execute(||{
            handle_conn(stream);
        });
    }
}

fn handle_conn(mut stream: TcpStream){

    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    println!(
        "Request: {}",
        String::from_utf8_lossy(&buffer[..])
    );

    let getreq= b"GET / HTTP/1.1\r\n";
    let postreq = b"POST /192.168.1.3:7878 HTTP/1.1\r\n";

    if buffer.starts_with(getreq){
        handle_get(buffer, stream);
    }
    else if buffer.starts_with(postreq){
        handle_post(buffer, stream);
    }
    else{
        println!("disgusting request");
    }
}

fn handle_get(buffer: [u8; 1024], mut stream: TcpStream){
    // stream.read(&mut buffer).unwrap();
    // println!(
    //     "Request: {}",
    //     String::from_utf8_lossy(&buffer[..])
    // );

    let getreq= b"GET / HTTP/1.1\r\n";

    let (status_line, filename) =
    if buffer.starts_with(getreq){
        ("HTTP/1.1 200 OK", "index.html")
    }
    else{
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    asnwer_std_get(status_line, filename, stream);
    
}

fn asnwer_std_get(status_line: &str, filename: &str, mut stream: TcpStream){
    let mut contents = fs::read_to_string(filename).unwrap();
    
    remove_html_tail(&mut contents);

    get_notes_for_index_html(&mut contents);

    add_html_tail(&mut contents);

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );
    
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}


fn handle_post(buffer: [u8; 1024], mut stream: TcpStream){
    println!("handle a post");
    let mut req_str = String::from_utf8_lossy(&buffer[..]);
    let mut req_str = req_str.to_string();
    
    // println!("{}", req_str);
    let mut a = req_str.split("\r\n\r\n");
    let b = a.next().unwrap();
    let b= a.next().unwrap();

    let mut a = String::from(b);
    let mut i = a.find("&").unwrap();

    let username = a[..i].to_string();
    let text = a[(i+1)..].to_string();
    
    let mut i = username.find("=").unwrap();
    let username = username[(i+1)..].to_string();
    // println!("{}", username); //OK
    let mut i = text.find("=").unwrap();
    let text = text[(i+1)..].to_string();
    let text = match decode(text.as_str()) {
        Ok(it) => it,
        Err(err) => panic!("explode"),
    };
    let text = text.replace("+", " ");
    let text = text.replace("\n", "");

    post_note (&username, &text);

    asnwer_std_get("HTTP/1.1 200 OK", "index.html", stream);
    
}

fn post_note(username:& String, text:& String){

    let mut file = fs::OpenOptions::new().write(true).append(true).open(NOTESPATH);
    let mut file = match file{
        Ok(f) => f,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => fs::File::create(NOTESPATH).unwrap(),
            _ => panic!("cannot open file"),
        },
    };

    let now = Utc::now();

    let app = format!("{};{};{}\n",username, text, now.format("%a %b %e %T %Y").to_string());
    write!(file, "{}",app);
}

fn remove_html_tail(s:&mut String){
    s.replace("</html>", "");
    s.replace("</body>", "");
}
fn add_html_tail(s:&mut String){
    s.push_str("</body>");
    s.push_str("</html>");
}


fn get_notes_for_index_html(htmlpage:&mut String) {

    let mut f = match fs::File::open(NOTESPATH) {
        Ok(file) => file,
        Err(e) => match e.kind(){
            ErrorKind::NotFound => fs::File::create(NOTESPATH).unwrap(),
            _ => panic!("problems opening the file"),
        },
    };

    let mut v: Vec<String> = Vec::new();
    let reader = BufReader::new(f);
    for line in reader.lines(){
        if let Ok(note) = line{
            
            //handle the csv
            let mut split = note.split(";");

            //get author
            let author = if let Some(aut) = split.next(){
                aut
            }
            else{
                panic!("wrong format");
            };
            // equal to: let author = split.next().unwrap();

            //get text
            let text = if let Some(t) = split.next(){
                t
            }
            else{
                panic!("wrong format");
            };
            // equal to: let text = split.next().unwrap();

            let date = if let Some(t) = split.next(){
                t
            }
            else{
                panic!("wrong format");
            };

            // println!("{} published: {}. ({})", author, text, date);
            let paragraph = format!("<p>{} published: {}. ({})<p>", author, text, date);
            v.push(paragraph.clone());
            // htmlpage.push_str(&paragraph[..]);
        }
        else{
            break;
        }
    }
    v.reverse();
    for line in v{
        htmlpage.push_str(&line[..]);
    }
}