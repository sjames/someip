use crate::error::{Error, Result};
use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;
#[derive(Debug)]
pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.

    pub fn from_bytes(input: &'de [u8]) -> Self {
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
    let mut deserializer = Deserializer::from_bytes(s);
    let t: T = T::deserialize(&mut deserializer)?;

    Ok(t)
}

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    // Look at the first character in the input without consuming it.

    // Parse a group of decimal digits as an unsigned integer of type T.

    fn parse_bool(&mut self) -> Result<bool> {
        //todo!()
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u8>());
        self.input = rest;
        let t = u8::from_ne_bytes(int_bytes.try_into().unwrap());
        if t == 0 {
            Ok(false)
        } else if t == 1 {
            Ok(true)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    fn parse_unsigned_u8(&mut self) -> Result<u8> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u8>());
        self.input = rest;
        let t = u8::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_unsigned_u16(&mut self) -> Result<u16> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u16>());
        self.input = rest;
        let t = u16::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_unsigned_u32(&mut self) -> Result<u32> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u32>());
        self.input = rest;
        let t = u32::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_unsigned_u64(&mut self) -> Result<u64> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<u64>());
        self.input = rest;
        let t = u64::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    // Parse a possible minus sign followed by a group of decimal digits as a
    // signed integer of type T.
    fn parse_signed_i8(&mut self) -> Result<i8> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i8>());
        self.input = rest;
        let t = i8::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_signed_i16(&mut self) -> Result<i16> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i16>());
        self.input = rest;
        let t = i16::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_signed_i32(&mut self) -> Result<i32> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i32>());
        self.input = rest;
        let t = i32::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_signed_i64(&mut self) -> Result<i64> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i64>());
        self.input = rest;
        let t = i64::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_float_f32(&mut self) -> Result<f32> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<f32>());
        self.input = rest;
        let t = f32::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    fn parse_float_f64(&mut self) -> Result<f64> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<f64>());
        self.input = rest;
        let t = f64::from_ne_bytes(int_bytes.try_into().unwrap());
        Ok(t)
    }

    // Parse a string until the next '"' character.
    //
    // Makes no attempt to handle escape sequences. What did you expect? This is
    // example code!
    fn parse_string(&mut self) -> Result<&'de str> {
        let (int_bytes, rest) = self.input.split_at(std::mem::size_of::<i32>());
        self.input = rest;

        let t = i32::from_ne_bytes(int_bytes.try_into().unwrap());

        let c: Vec<u8> = self.input.to_vec();
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
    }

    fn parse_char(&mut self) -> Result<char> {
        let str2: &str = std::str::from_utf8(self.input).unwrap();
        let my_char = str2.chars().next().expect("string is empty");
        self.parse_unsigned_u8();
        Ok(my_char)
    }
}

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

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

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

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        _visitor.visit_f32(self.parse_float_f32()?)
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        _visitor.visit_f64(self.parse_float_f64()?)
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse a string, check that it is one character, call `visit_char`.

        _visitor.visit_char(self.parse_char()?)
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

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)
    }

    // As commented in `Serializer` implementation, this is a lossy
    // representation.

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with(b"null") {
            visitor.visit_none()
        } else {
            Err(Error::Syntax)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with(b"null") {
            visitor.visit_unit()
        } else {
            Err(Error::Syntax)
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
        let d: Vec<u8> = self.input.to_vec();
        if (d.len() > 4) | (d.len() == 4) {
            let m = &d[0..4];
            let n = &d[4..d.len()];
            let t = i32::from_ne_bytes(m.try_into().unwrap());

            if (t == n.len().try_into().unwrap()) | (t < n.len().try_into().unwrap()) {
                let value = _visitor.visit_seq(SomeIpVec::new(self))?;

                Ok(value)
            } else {
                let value = _visitor.visit_seq(SomeIp::new(self))?;

                Ok(value)
            }
        } else {
            let value = _visitor.visit_seq(SomeIp::new(self))?;

            Ok(value)
        }
    }

    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

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

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse the opening brace of the map.

        self.deserialize_seq(_visitor)
    }
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
    // the variant of an enum.
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

struct SomeIp<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> SomeIp<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        SomeIp { de, first: true }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for SomeIp<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // Check if there are no more elements.
        _seed.deserialize(&mut *self.de).map(Some)
    }
}

struct SomeIpVec<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
    len: usize,
}
impl<'a, 'de> SomeIpVec<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        let (int_bytes, rest) = de.input.split_at(std::mem::size_of::<i32>());
        de.input = rest;
        let len = i32::from_ne_bytes(int_bytes.try_into().unwrap());

        SomeIpVec {
            de,
            first: true,
            len: len.try_into().unwrap(),
        }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for SomeIpVec<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // Check if there are no more elements.

        if self.len > 0 {
            self.len -= 1;

            _seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }

        // `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
        // through elements of the seq
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for SomeIp<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, _seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.
        _seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, _seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // It doesn't make a difference whether the colon is parsed at the end
        // of `next_key_seed` or at the beginning of `next_value_seed`. In this
        // case the code is a bit simpler having it here.

        _seed.deserialize(&mut *self.de)
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
        Err(Error::Syntax)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        Err(Error::Syntax)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Syntax)
    }
}
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            bool: bool,
            i: u8,
            ch: String,
            tup: [u8; 2],
            bool_ch: bool,
            t: u16,
            s: u8,
            seq: Vec<String>,
            seq2: Vec<u8>,
            seq3: Vec<Vec<u8>>,
        }

        let j: [u8; 62] = [
            0, 2, 5, 0, 0, 0, 239, 187, 191, 72, 105, 0, 6, 4, 1, 1, 0, 9, 2, 0, 0, 0, 4, 0, 0, 0,
            239, 187, 191, 97, 0, 4, 0, 0, 0, 239, 187, 191, 98, 0, 2, 0, 0, 0, 12, 32, 2, 0, 0, 0,
            2, 0, 0, 0, 12, 34, 2, 0, 0, 0, 23, 43,
        ];

        let expected = Test {
            bool: false,
            i: 2u8,
            ch: "Hi".to_owned(),
            tup: [6, 4],
            bool_ch: true,
            t: 1u16,
            s: 9u8,
            seq: vec!["a".to_owned(), "b".to_owned()],
            seq2: vec![12, 32],
            seq3: vec![vec![12, 34], vec![23, 43]],
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
        let j: [u8; 1] = [255];
        assert_eq!(-1i8, from_bytes(&j).unwrap());
    }

    #[test]
    fn test_f32() {
        let j: [u8; 4] = [92, 35, 19, 69];
        assert_eq!(2354.21f32, from_bytes(&j).unwrap());
    }
    #[test]
    fn test_f64() {
        let j: [u8; 8] = [82, 184, 30, 133, 107, 100, 162, 64];
        assert_eq!(2354.21f64, from_bytes(&j).unwrap());
    }

    #[test]
    fn test_bool() {
        let j = [1];
        assert_eq!(true, from_bytes(&j).unwrap());
    }
    #[test]
    fn test_string() {
        let j: [u8; 10] = [5, 0, 0, 0, 239, 187, 191, 72, 105, 0];

        assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    }

    #[test]
    fn test_string_unequal_len() {
        let j: [u8; 10] = [7, 0, 0, 0, 239, 187, 191, 72, 105, 0];
        assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    }
    #[test]
    fn test_string_no_terminate() {
        let j: [u8; 9] = [5, 0, 0, 0, 239, 187, 191, 72, 105];
        assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    }

    #[test]
    fn test_string_no_bom() {
        let j: [u8; 7] = [5, 0, 0, 0, 72, 105, 0];

        assert_eq!("Hi", from_bytes::<&str>(&j).unwrap());
    }

    #[test]
    fn test_tupule() {
        let j: [u8; 3] = [6, 8, 9];
        assert_eq!((6u8, 8u8, 9u8), from_bytes(&j).unwrap());
    }
    #[test]
    fn test_char() {
        let j: [u8; 1] = [97];
        assert_eq!('a', from_bytes::<char>(&j).unwrap());
    }

    #[test]
    fn test_seq() {
        let test = vec![1, 2, 9, 7];
        let expected = vec![4, 0, 0, 0, 1, 2, 9, 7];
        let j = expected.as_ref();
        assert_eq!(test, from_bytes::<Vec<u8>>(&j).unwrap());
    }

    #[test]
    fn test_nested_seq() {
        let test: Vec<Vec<u8>> = vec![vec![1, 3, 4, 6], vec![3, 4, 9, 1]];
        let expected: Vec<u8> = vec![2, 0, 0, 0, 4, 0, 0, 0, 1, 3, 4, 6, 4, 0, 0, 0, 3, 4, 9, 1];
        let j = expected.as_ref();
        assert_eq!(test, from_bytes::<Vec<Vec<u8>>>(&j).unwrap());
    }

    #[test]
    fn test_vec_str() {
        let test = vec!["a".to_owned(), "b".to_owned()];
        let expected = vec![
            2, 0, 0, 0, 4, 0, 0, 0, 239, 187, 191, 97, 0, 4, 0, 0, 0, 239, 187, 191, 98, 0,
        ];
        let j = expected.as_ref();
        assert_eq!(test, from_bytes::<Vec<String>>(&j).unwrap());
    }
}
