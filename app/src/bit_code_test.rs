use std::io::Write;
use std::time::Instant;

use bitcode::__private::{Decoder, View};
use bitcode::{Buffer, Decode, Encode};

use crate::net::Message;

#[derive(Encode, Decode, PartialEq, Debug)]
struct Foo<'a> {
    x: u32,
    y: &'a str,
}

pub(crate) fn test_bitcode() {
    let mut original = Foo { x: 10, y: "abc" };
    let m1 = Message::Ping {
        time: Instant::now().elapsed().as_secs_f64(),
    };
    let m2 = Message::ServerInfo {
        key: vec![1u8, 2, 3, 4, 5, 6, 7, 8],
    };
    let m3 = Message::Hello;
    let mut buffer = Buffer::new();

    let mut buf: Vec<u8> = Vec::new();

    buf.write(buffer.encode(&m3)).unwrap();
    // buf.write(buffer.encode(&m1)).unwrap();
    // buf.write(buffer.encode(&m2)).unwrap();
    // buf.write(buffer.encode(&m1)).unwrap();
    // buf.write(buffer.encode(&m2)).unwrap();
    println!("Buf={}", buf.len());

    let mut decoder = <Message<'_> as bitcode::Decode>::Decoder::default();
    let s = &mut buf.as_slice();
    while !s.is_empty() {
        //let result: Message = buffer.decode(s).unwrap();
        decoder.populate(s, 1).unwrap();
        let result = decoder.decode();
        println!("{}, Got {:?}", s.len(), result);
    }
    println!("Messages decoded.");
    buf.clear();
    for i in 0..3 {
        let encoded: Vec<u8> = bitcode::encode(&original); // No error
        original.x += 10;
        buf.write(encoded.as_slice()).expect("A!");
    }
    let mut decoder = <Foo<'_> as bitcode::Decode>::Decoder::default();
    let s = &mut buf.as_slice();
    println!("Buf={}", s.len());
    while !s.is_empty() {
        decoder.populate(s, 1).unwrap();
        //expect_eof(bytes)?;
        let result = decoder.decode(); // decode_inline_never(&mut decoder);
        println!("{}, Got {:?}", s.len(), result);
        //let decoded: Foo<'_> = bitcode::decode(&buf).unwrap();
    }
    println!("All done!");
    //assert_eq!(original, decoded);
}
