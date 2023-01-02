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

    let expected = vec![
        98, 0, 79, 16, 171, 67, 99, 7, 0, 0, 0, 239, 187, 191, 116, 101, 115, 116, 0, 1, 3,
    ];
    let len: i32 = expected.len().try_into().unwrap();
    println!("{}", len);
    //let j:&[u8]=&expected;
    let k: &[u8] = expected.as_ref();
    println!("{:?}", k);
    // assert_eq!(to_bytes(&test).unwrap(), expected);

    assert_eq!(test, from_bytes(k).unwrap());
}

#[test]
fn test_bool() {
    let test1: bool = true;
    let expected = vec![0x1];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_i8() {
    let test1: i8 = 0x12;
    let expected = vec![0x12];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_i16() {
    let test1: i16 = 0x1234;
    let expected = vec![0x34, 0x12];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_i32() {
    let test1: i32 = 0x12345678;
    let expected = vec![0x78, 0x56, 0x34, 0x12];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_u8() {
    let test1: u8 = 0x12;
    let expected = vec![0x12];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_u16() {
    let test1: u16 = 0x1234;
    let expected = vec![0x34, 0x12];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_u32() {
    let test1: u32 = 0x12345678;
    let expected = vec![0x78, 0x56, 0x34, 0x12];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_f32() {
    let test1: f32 = 2354.21;
    let expected = vec![92, 35, 19, 69];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_f64() {
    let test1: f64 = 2354.21;
    let expected = vec![82, 184, 30, 133, 107, 100, 162, 64];
    assert_eq!(to_bytes(&test1).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test1, from_bytes(&j).unwrap());
}

#[test]
fn test_string() {
    let str: String = String::from("Hi");
    let expected = vec![5, 0, 0, 0, 239, 187, 191, 72, 105, 0];
    assert_eq!(to_bytes(&str).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(&str, from_bytes::<&str>(&j).unwrap());
}

#[test]
fn test_tuple() {
    let test: (u8, u8, u8) = (1, 2, 3);
    let expected: Vec<u8> = vec![1, 2, 3];
    assert_eq!(to_bytes(&test).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test, from_bytes(&j).unwrap());
}

#[test]
fn test_nested_tuple() {
    let test: ((u8, u8), (u8, u8)) = ((1, 2), (2, 3));
    let expected: Vec<u8> = vec![1, 2, 2, 3];
    assert_eq!(to_bytes(&test).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test, from_bytes(&j).unwrap());
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

    let expected = vec![12, 2, 43, 9];
    assert_eq!(to_bytes(&test).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test, from_bytes(&j).unwrap());
}

////added the code for char in de.rs
#[test]
fn test_char() {
    let char: char = 'V';
    let mut ex = vec![];
    ex.push(char as u8);
    assert_eq!(to_bytes(&char).unwrap(), ex);
    let j = ex.as_ref();
    assert_eq!(char, from_bytes::<char>(&j).unwrap());
}

/////test case for test_seq and test_nested_seq have failed
#[test]
fn test_seq() {
    let test: Vec<u8> = vec![1, 2];
    let expected = vec![1, 2];
    assert_eq!(to_bytes(&test).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test, from_bytes::<Vec<u8>>(&j).unwrap());
}

#[test]
fn test_nested_seq() {
    let test: Vec<Vec<u8>> = vec![vec![1, 3], vec![3, 4], vec![5, 6]];
    let expected: Vec<u8> = vec![1, 3, 3, 4, 5, 6];
    assert_eq!(to_bytes(&test).unwrap(), expected);
    let j = expected.as_ref();
    assert_eq!(test, from_bytes::<Vec<Vec<u8>>>(&j).unwrap());
}
