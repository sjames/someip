use serde::{ser, Serialize};

use crate::{
    de::from_bytes,
    error::{Error, Result},
    ser::to_bytes,
};
use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;
#[test]
fn test_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Test {
        int: u8,
        bool: bool,
        float: f32,
        char: char,
        str: String,
        tup: (u8, u8),
        //seq: Vec<&'static str>,
        //seq2: Vec<u8>,
        //seq3: Vec<Vec<u8>>,
    }

    let test = Test {
        int: 98,
        bool: false,
        float: 342.1274,
        char: 'c',
        str: String::from("test"),
        tup: (1, 3),
        //seq: vec!["a", "b"],
        //seq2: vec![12, 32],
        //seq3: vec![vec![12, 34], vec![23, 43]],
    };

    let ser = to_bytes(&test).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test, der);
}

#[test]
fn test_bool() {
    let test1: bool = true;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_i8() {
    let test1: i8 = 0x12;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_i16() {
    let test1: i16 = 0x1234;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_i32() {
    let test1: i32 = 0x12345678;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_u8() {
    let test1: u8 = 0x12;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_u16() {
    let test1: u16 = 0x1234;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_u32() {
    let test1: u32 = 0x12345678;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_f32() {
    let test1: f32 = 2354.21;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_f64() {
    let test1: f64 = 2354.21;
    let ser = to_bytes(&test1).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test1, der);
}

#[test]
fn test_string() {
    let str: String = String::from("Hi");
    let ser = to_bytes(&str).unwrap();
    let j = ser.as_ref();
    let der = from_bytes::<String>(&j).unwrap();
    assert_eq!(str, der);
}

#[test]
fn test_tuple() {
    let test: (u8, u8, u8) = (1, 2, 3);
    let ser = to_bytes(&test).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test, der);
}

#[test]
fn test_nested_tuple() {
    let test: ((u8, u8), (u8, u8)) = ((1, 2), (2, 3));
    let ser = to_bytes(&test).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test, der);
}
#[test]
fn test_struct_array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Test {
        seq: [u8; 4],
    }

    let test = Test {
        seq: [12, 2, 43, 9],
    };

    let ser = to_bytes(&test).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(test, der);
}

////added the code for char in de.rs
#[test]
fn test_char() {
    let ch: char = 'V';
    let ser = to_bytes(&char).unwrap();
    let j = ser.as_ref();
    let der = from_bytes(&j).unwrap();
    assert_eq!(ch, der);
}

/////test case for test_seq and test_nested_seq have failed
#[test]
fn test_seq() {
    let test: Vec<u8> = vec![1, 2];
    let ser = to_bytes(&test).unwrap();
    let j = ser.as_ref();
    let der = from_bytes::<Vec<u8>>(&j).unwrap();
    assert_eq!(test, der);
}

#[test]
fn test_nested_seq() {
    let test: Vec<Vec<u8>> = vec![vec![1, 3], vec![3, 4], vec![5, 6]];
    let ser = to_bytes(&test).unwrap();
    let j = ser.as_ref();
    let der = from_bytes::<Vec<Vec<u8>>>(&j).unwrap();
    assert_eq!(test, der);
}
