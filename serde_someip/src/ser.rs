use serde::{ser, Serialize};

use crate::error::{Error, Result};

pub struct Serializer {
    output: Vec<u8>,
}


pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer{
        output: Vec::new(),
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}



impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    // Here we go with the simple methods. The following 12 methods receive one
    // of the primitive types of the data model and map it to the output by appending
    // into the output string.

    fn serialize_bool(self, v: bool) -> Result<()> {
        Ok(self.output.push(u8::from(v)))
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        Ok(self.output.push(v as u8))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        Ok(self.output.extend(v.to_ne_bytes()))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        Ok(self.output.extend(v.to_ne_bytes()))
    }

    fn serialize_i64(self, v: i64) -> Result<()>{
        Err(Error::Syntax)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        Ok(self.output.push(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        Ok(self.output.extend(v.to_ne_bytes()))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        Ok(self.output.extend(v.to_ne_bytes()))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        Err(Error::Syntax)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        Ok(self.output.extend(v.to_ne_bytes()))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        Ok(self.output.extend(v.to_ne_bytes()))
    }

    fn serialize_char(self, v: char) -> Result<()> {
        let res = v as u8;
        Ok(self.output.push(res))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        Ok(self.output.extend(v.as_bytes()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        Ok(self.output.extend(v))
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::Syntax)
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    // In Serde, unit means an anonymous value containing no data. 
    fn serialize_unit(self) -> Result<()> {
        Err(Error::Syntax)
    }

    // Unit struct means a named value containing no data. Again, since there is
    // no data. There is no need to serialize the
    // name in most formats.
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(Error::Syntax)
    }

    // When serializing a unit variant (or any other kind of variant), formats
    // can choose whether to keep track of it by index or by name. Binary
    // formats typically use the index of the variant and human-readable formats
    // typically use the name.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        Err(Error::Syntax)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain.
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    // Note that newtype variant (and all of the other variant serialization
    // methods) refer exclusively to the "externally tagged" enum
    // representation.
    //
    
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    // Now we get to the serialization of compound types.
    //
    // The start of the sequence, each value, and the end are three separate
    // method calls.The length of the sequence may or may not be known ahead of time. This
    // doesn't make a difference in JSON because the length is not represented
    // explicitly in the serialized form. Some serializers may only be able to
    // support sequences for which the length is known up front.
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    // Some formats may be able to represent tuples more efficiently by omitting the length, since tuple
    // means that the corresponding `Deserialize implementation will know the
    // length without needing to look at the serialized data.
    fn serialize_tuple(self, len: usize) -> Result<(Self::SerializeTuple)> {
        Ok(self)
    }

    // Tuple structs look just like sequences 
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::Syntax)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }`. Again
    // this method is only responsible for the externally tagged representation.
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::Syntax)
    }

    // Maps are represented in JSON as `{ K: V, K: V, ... }`.
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::Syntax)
    }

    // Structs look just like maps in JSON. In particular, JSON requires that we
    // serialize the field names of the struct. Other formats may be able to
    // omit the field names when serializing structs because the corresponding
    // Deserialize implementation is required to know what the keys are without
    // looking at the serialized data.
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<(Self::SerializeStruct)> {
        Ok(self)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }`.
    // This is the externally tagged representation.
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::Syntax)
        
    }
}

// The following 7 impls deal with the serialization of compound types like
// sequences and maps. Serialization of such types is begun by a Serializer
// method and followed by zero or more calls to serialize individual elements of
// the compound type and one call to end the compound type.
//


// This impl is SerializeSeq so these methods are called after `serialize_seq`
// is called on the Serializer.
impl<'a> ser::SerializeSeq for &'a mut Serializer {
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {  
        value.serialize(&mut **self) 
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Same thing but for tuples.
impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {       
        value.serialize(&mut **self)

    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    fn end(self) -> Result<()> {
        Err(Error::Syntax)
    }
}

// Tuple variants are a little different. Refer back to the
// `serialize_tuple_variant` method above:
//
//    self.output += "{";
//    variant.serialize(&mut *self)?;
//    self.output += ":[";
//
// So the `end` method in this impl is responsible for closing both the `]` and
// the `}`.

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    fn end(self) -> Result<()> {
        Err(Error::Syntax)
    }
}

// Some `Serialize` types are not able to hold a key and value in memory at the
// same time so `SerializeMap` implementations are required to support
// `serialize_key` and `serialize_value` individually.
//
// There is a third optional method on the `SerializeMap` trait. The
// `serialize_entry` method allows serializers to optimize for the case where
// key and value are both available simultaneously. In JSON it doesn't make a
// difference so the default behavior for `serialize_entry` is fine.
impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    // It doesn't make a difference whether the colon is printed at the end of
    // `serialize_key` or at the beginning of `serialize_value`. In this case
    // the code is a bit simpler having it here.
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    fn end(self) -> Result<()> {
        Err(Error::Syntax)
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Syntax)
    }

    fn end(self) -> Result<()> {
        Err(Error::Syntax)
    }
}



#[cfg(test)]
mod tests {

    use std::vec;

    use super::*;

    #[test]
    fn test_bool(){
        let test1:bool = true;
        let expected = vec![0x1];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }

    #[test]
    fn test_i8(){
        let test1:i8 = 0x12;
        let expected = vec![0x12];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }

    #[test]
    fn test_i16(){
        let test1:i16 = 0x1234;
        let expected = vec![0x34,0x12];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }

    #[test]
    fn test_i32(){
        let test1:i32 = 0x12345678;
        let expected = vec![0x78,0x56,0x34,0x12];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }

    #[test]
    fn test_u8(){
        let test1:u8 = 0x12;
        let expected = vec![0x12];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }
    #[test]
    fn test_u16(){
        let test1:u16 = 0x1234;
        let expected = vec![0x34,0x12];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }

    #[test]
    fn test_u32(){
        let test1:u32 = 0x12345678;
        let expected = vec![0x78,0x56,0x34,0x12];
        assert_eq!(to_bytes(&test1).unwrap(),expected);
    }

    #[test]
    fn test_string(){
        let str:String = String::from("Serializing");
        let expected = str.as_bytes();
        assert_eq!(to_bytes(&str).unwrap(),expected);
    }

    #[test]
    fn test_char(){
        let char: char = 'V';
        let mut ex = vec![];
        ex.push(char as u8);
        assert_eq!(to_bytes(&char).unwrap(),ex);
    }

    #[test]
    fn test_seq(){
        let test:Vec<u8> = vec![1,2];
        let expected =vec![1,2];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }

    #[test]
    fn test_nested_seq(){
        let test:Vec<Vec<u8>> = vec![vec![1,3],vec![3,4],vec![5,6]];
        let expected:Vec<u8> = vec![1,3,3,4,5,6];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }

    #[test]
    fn test_tuple(){
        let test:(u8,u8,u8) = (1,2,3);
        let expected:Vec<u8> = vec![1,2,3];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }

    #[test]
    fn test_nested_tuple(){
        let test:((u8,u8),(u8,u8))=((1,2),(2,3));
        let expected:Vec<u8> = vec![1,2,2,3];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }


    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct Test {
            int: u8,
            bool:bool,
            float:f32,
            char:char,
            str:String,
            tup:(u8,u8),
            seq: Vec<&'static str>,
            seq2: Vec<u8>,
            seq3: Vec<Vec<u8>>,
            
        }

        let test = Test {
            int: 98,
            bool: false,
            float: 342.1274,
            char: 'c',
            str:String::from("test"),
            tup:(1,3),
            seq: vec!["a", "b"],
            seq2: vec![12,32],
            seq3:vec![vec![12,34],vec![23,43]]
            
        };
        
        let expected = vec![98,0,79,16,171,67,99,116, 101, 115, 116,1,3,97,98,12,32,12,34,23,43];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }

    #[test]
    fn test_struct_array(){
        #[derive(Serialize)]
        struct Test {
            seq: [u8;4],
        }

        let test = Test {
            seq:[12,2,43,9]
            
        };
        
        let expected = vec![12,2,43,9];
        assert_eq!(to_bytes(&test).unwrap(), expected);
    }
}
 
