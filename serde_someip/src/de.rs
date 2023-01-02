use crate::error::{Error, Result};
use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;
#[derive(Debug)]
pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de [u8],
    //input: & 'de Vec<u8>,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    pub fn from_bytes(input: &'de [u8]) -> Self {
        println!("from_bytes from deserializer");

        Deserializer { input }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    println!("from bytes");

    let mut deserializer = Deserializer::from_bytes(s);
    let t: T = T::deserialize(&mut deserializer)?;

    Ok(t)
}

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    // Look at the first character in the input without consuming it.

    // Parse the JSON identifier `true` or `false`.

    // Parse a group of decimal digits as an unsigned integer of type T.
    //
    // This implementation is a bit too lenient, for example `001` is not
    // allowed in JSON. Also the various arithmetic operations can overflow and
    // panic or return bogus data. But it is good enough for example code!

    fn parse_bool(&mut self) -> Result<bool> {
        //todo!()

        //let mut deserializer = Deserializer::from_bytes(self.input);
        //let t: T = T::deserialize(&mut deserializer)?;
        // match u8::deserialize(&mut deserializer)? {
        //     0 => Ok(false),
        //     1 => Ok(true),
        //     _x => Err(Error::ExpectedBoolean),
        // }
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u8>());
        self.input = rest;
        let t = u8::from_ne_bytes(int_bytes.try_into().unwrap());
        if t == 0 {
            Ok(false)
        } else if t == 1 {
            Ok(true)
        } else {
            //Ok(false)
            Err(Error::ExpectedBoolean)
        }
        //println!("{:?}",int_bytes);
    }

    fn parse_unsigned_u8(&mut self) -> Result<u8> {
        //let mid=std::mem::size_of::<u8>();

        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u8>());

        self.input = rest;
        println!("{:?}", rest);
        let t = u8::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    //let t=self.input.to_vec().pop().unwrap();

    //Ok(t)

    fn parse_unsigned_u16(&mut self) -> Result<u16> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u16>());
        self.input = rest;
        println!("{:?}", rest);
        let t = u16::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_unsigned_u32(&mut self) -> Result<u32> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u32>());
        self.input = rest;
        println!("{:?}", rest);
        let t = u32::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_unsigned_u64(&mut self) -> Result<u64> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u64>());
        self.input = rest;
        println!("{:?}", rest);
        let t = u64::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    // Parse a possible minus sign followed by a group of decimal digits as a
    // signed integer of type T.
    fn parse_signed_i8(&mut self) -> Result<i8> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i8>());
        self.input = rest;
        println!("{:?}", rest);
        let t = i8::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_signed_i16(&mut self) -> Result<i16> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i16>());
        self.input = rest;
        println!("{:?}", rest);
        let t = i16::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_signed_i32(&mut self) -> Result<i32> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i32>());
        self.input = rest;
        println!("{:?}", rest);
        let t = i32::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_signed_i64(&mut self) -> Result<i64> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i64>());
        self.input = rest;
        println!("{:?}", rest);
        let t = i64::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_float_f32(&mut self) -> Result<f32> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<f32>());
        self.input = rest;
        println!("{:?}", rest);
        let t = f32::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    fn parse_float_f64(&mut self) -> Result<f64> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<f64>());
        self.input = rest;
        println!("{:?}", rest);
        let t = f64::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        Ok(t)
    }

    // Parse a string until the next '"' character.
    //
    // Makes no attempt to handle escape sequences. What did you expect? This is
    // example code!
    fn parse_string(&mut self) -> Result<&'de str> {
        //todo!()
        //let len=&self.input[0..32];

        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i32>());
        self.input = rest;
        println!("{:?}", rest);
        let t = i32::from_ne_bytes(int_bytes.try_into().unwrap());
        println!("{:?}", int_bytes);
        let c: Vec<u8> = self.input.to_vec();

        //n => self.deserialize_unit(visitor),
        if c.iter().any(|&c| c == 0) {
            let slice_index_element = self
                .input
                .iter()
                .position(|&x| x == "\0".as_bytes()[0])
                .unwrap();
            let mut b = self.input.iter();
            let seq = (b.next(), b.next(), b.next());
            match seq {
                (Some(239), Some(187), Some(191)) => {
                    let str2: &str = std::str::from_utf8(self.input).unwrap();

                    let first_last_off: &str = &str2[0..slice_index_element];

                    if first_last_off.len() == t.try_into().unwrap() {
                        let str2: &str = std::str::from_utf8(self.input).unwrap();
                        let first_last_off: &str = &str2[3..slice_index_element];

                        for _i in 0..slice_index_element + 1 {
                            self.parse_unsigned_u8();
                        }
                        return Ok(first_last_off);
                    } else {
                        return Err(Error::Syntax);
                    }
                }

                _ => Err(Error::Syntax),
            }
        } else {
            return Err(Error::ExpectedNull);
        }
        //n => self.deserialize_unit(visitor),
    }

    fn parse_char(&mut self) -> Result<char> {
        let str2: &str = std::str::from_utf8(self.input).unwrap();
        //let x = i32::from_str(str2).unwrap();
        //let first_last_off: &str = &str2[3..str2.len() - 1];
        //println!("{}", first_last_off);

        let my_char = str2.chars().next().expect("string is empty");
        self.parse_unsigned_u8();
        Ok(my_char)
    }
}

// fn parse_string(&mut self) -> Result<&'de str> {
// let str2: &str = std::str::from_utf8(self.input).unwrap();
// println!("{:?}", str2);
// //Ok(first_last_off)
// Ok(str2)

// }

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)
    }

    // Uses the `parse_bool` parsing function defined above to read the JSON
    // identifier `true` or `false` from the input.
    //
    // Parsing refers to looking at the input and deciding that it contains the
    // JSON value `true` or `false`.
    //
    // Deserialization refers to mapping that JSON value into Serde's data
    // model by invoking one of the `Visitor` methods. In the case of JSON and
    // bool that mapping is straightforward so the distinction may seem silly,
    // but in other cases Deserializers sometimes perform non-obvious mappings.
    // For example the TOML format has a Datetime type and Serde's data model
    // does not. In the `toml` crate, a Datetime in the input is deserialized by
    // mapping it to a Serde data model "struct" type with a special name and a
    // single field containing the Datetime represented as a string.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let _string: &str = "bool";
        visitor.visit_bool(self.parse_bool()?)

        //Ok(string)
    }

    // The `parse_https://github.com/bincode-org/bincode/blob/trunk/src/de/impls.rssigned` function is generic over the integer type `T` so here
    // it is invoked with `T=i8`. The next 8 methods are similar.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_signed_i8()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_signed_i16()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_signed_i32()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned_u8()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned_u16()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned_u32()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_unsigned_u64()?)
    }

    // Float parsing is stupidly hard.
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        _visitor.visit_f32(self.parse_float_f32()?)
        //unimplemented!()
    }

    // Float parsing is stupidly hard.
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        _visitor.visit_f64(self.parse_float_f64()?)
        //unimplemented!()
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse a string, check that it is one character, call `visit_char`.
        //unimplemented!()
        _visitor.visit_char(self.parse_char()?)
        //Err(Error::Syntax)
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)

        //unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        //_visitor.visit_u8(self.parse_unsigned_u8()?)
        //self.deserialize_bytes(_visitor)
        Err(Error::Syntax)
        //unimplemented!()
    }

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with(b"null") {
            //self.input = &self.input["null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with(b"null") {
            //self.input = &self.input["null".len()..];
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse the opening bracket of the sequence.
        let value = _visitor.visit_seq(CommaSeparated::new(self))?;

        //println!("Yes");
        Ok(value)
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        println!("deserialize_tupule");
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse the opening brace of the map.

        self.deserialize_seq(_visitor)
        // let value = _visitor.visit_map(CommaSeparated::new(self))?;
        // // Parse the closing brace of the map.

        // Ok(value)
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,

        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

// In order to handle commas correctly when deserializing a JSON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated { de, first: true }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // Check if there are no more elements.
        _seed.deserialize(&mut *self.de).map(Some)

        // `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
        // through elements of the seq
        //todo!()
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, _seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.

        _seed.deserialize(&mut *self.de).map(Some)
        //todo!()
    }

    fn next_value_seed<V>(&mut self, _seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // It doesn't make a difference whether the colon is parsed at the end
        // of `next_key_seed` or at the beginning of `next_value_seed`. In this
        // case the code is a bit simpler having it here.

        _seed.deserialize(&mut *self.de)

        //todo!()
    }
}
struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}
// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // The `deserialize_enum` method parsed a `{` character so we are
        // currently inside of a map. The seed will be deserializing itself from
        // the key of the map.
        let _val = seed.deserialize(&mut *self.de)?;
        // Parse the colon separating map key from value.

        Err(Error::Syntax)
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::ExpectedString)
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}
////////////////////////////////////////////////////////////////////////////////

#[test]
fn test_struct() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Test<'a> {
        bool: bool,
        i: u8,
        ch: &'a str,
        tup: [u8; 2],
        bool_ch: bool,
        t: u16,
        s: u8,
    }

    let j: [u8; 18] = [
        0, 2, 5, 0, 0, 0, 239, 187, 191, 72, 105, 0, 6, 4, 1, 1, 0, 9,
    ];

    let expected = Test {
        bool: false,
        i: 2u8,
        ch: "Hi",
        tup: [6, 4],
        bool_ch: true,
        t: 1u16,
        s: 9u8,
    };
    assert_eq!(expected, from_bytes(&j).unwrap());
}

#[test]
fn test_u8() {
    let j: [u8; 1] = [9];
    assert_eq!(9u8, from_bytes(&j).unwrap());
}
#[test]
fn test_u16() {
    let j: [u8; 2] = [1, 0];
    assert_eq!(1u16, from_bytes(&j).unwrap());
}

#[test]
fn test_i8() {
    //let i: u8=9;
    let j: [u8; 1] = [255];
    assert_eq!(-1i8, from_bytes(&j).unwrap());
}

#[test]
fn test_f32() {
    //let i: u8=9;
    let j: [u8; 4] = [92, 35, 19, 69];
    assert_eq!(2354.21f32, from_bytes(&j).unwrap());
}
#[test]
fn test_f64() {
    //let i: u8=9;
    let j: [u8; 8] = [82, 184, 30, 133, 107, 100, 162, 64];
    assert_eq!(2354.21f64, from_bytes(&j).unwrap());
}

#[test]
fn test_bool() {
    //let j = b"0";
    //let i: u8=0;
    let j = [1];
    assert_eq!(true, from_bytes(&j).unwrap());
}
#[test]
fn test_string() {
    //let j = b"0";
    //let i = String::from("Done");
    let j: [u8; 10] = [5, 0, 0, 0, 239, 187, 191, 72, 105, 0];
    //let j: [u8;4] = [2,72,105,0];
    assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    //let j: [u8;4] = [5,72,105,0];
}

#[test]
fn test_string_unequal_len() {
    //let j = b"0";
    //let i = String::from("Done");
    let j: [u8; 10] = [7, 0, 0, 0, 239, 187, 191, 72, 105, 0];
    //let j: [u8;4] = [2,72,105,0];
    assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    //let j: [u8;4] = [5,72,105,0];
}
#[test]
fn test_string_no_terminate() {
    //let j = b"0";
    //let i = String::from("Done");
    let j: [u8; 9] = [5, 0, 0, 0, 239, 187, 191, 72, 105];
    //let j: [u8;4] = [2,72,105,0];
    assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    //let j: [u8;4] = [5,72,105,0];
}

#[test]
fn test_string_no_bom() {
    //let j = b"0";
    //let i = String::from("Done");
    let j: [u8; 7] = [5, 0, 0, 0, 72, 105, 0];
    //let j: [u8;4] = [2,72,105,0];
    assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    //let j: [u8;4] = [5,72,105,0];
}

#[test]
fn test_tupule() {
    //let j = b"0";

    let j: [u8; 14] = [6, 8, 4, 0, 0, 0, 239, 187, 191, 97, 0, 9, 1, 0];
    //let k:[u8;2] = [,];
    assert_eq!((6u8, 8u8, "a", 9u8, 1u16), from_bytes(&j).unwrap());
}
#[test]
fn test_char() {
    let j: [u8; 1] = [97];
    //let k:[u8;2] = [,];
    assert_eq!('a', from_bytes::<char>(&j).unwrap());
}

////test case for test_seq and test_nested_seq have failed
#[test]
fn test_seq() {
    let test: Vec<u8> = vec![1, 2];
    let expected = vec![1, 2];
    let j = expected.as_ref();
    assert_eq!(test, from_bytes::<Vec<u8>>(&j).unwrap());
}

#[test]
fn test_nested_seq() {
    let test: Vec<Vec<u8>> = vec![vec![1, 3], vec![3, 4], vec![5, 6]];
    let expected: Vec<u8> = vec![1, 3, 3, 4, 5, 6];
    let j = expected.as_ref();
    assert_eq!(test, from_bytes::<Vec<Vec<u8>>>(&j).unwrap());
}
