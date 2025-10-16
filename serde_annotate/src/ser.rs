use std::cell::Cell;

use serde::ser;

use crate::annotate::{Annotate, Format, MemberId};
use crate::document::{BytesFormat, CommentFormat, Document, StrFormat};
use crate::error::Error;
use crate::hexdump;
use crate::integer::{Base, Int};

pub fn serialize<T>(value: &T) -> Result<Document, Error>
where
    T: ?Sized + ser::Serialize,
{
    value.serialize(&mut AnnotatedSerializer::new())
}

/// Serializer adapter that adds user-annotatons to the serialized document.
#[derive(Clone)]
pub struct AnnotatedSerializer {
    base: Base,
    strformat: StrFormat,
    bytesformat: BytesFormat,
    compact: bool,
}

thread_local! {
    static ANNOTATE: Cell<Option<&'static dyn Annotate>> = const { Cell::new(None) };
}

impl AnnotatedSerializer {
    pub fn new() -> Self {
        AnnotatedSerializer {
            base: Base::Dec,
            strformat: StrFormat::Standard,
            bytesformat: BytesFormat::Standard,
            compact: false,
        }
    }

    /// Provide an annotator to inform how to annotate the serialization of the current object.
    pub fn with<T>(value: Option<&dyn Annotate>, f: impl FnOnce(Option<&dyn Annotate>) -> T) -> T {
        ANNOTATE.with(|annotate| {
            // SAFETY: use `transmute` to erase the lifetime. This is sound as we use a scope guard
            // to ensure that the value is restored when the function returns, so we do not extend
            // the lifetime beyond this function.
            let old = annotate.replace(unsafe { std::mem::transmute(value) });
            scopeguard::defer! {
                annotate.set(old);
            }
            f(old)
        })
    }

    fn with_base(&self, b: Base) -> Self {
        let mut x = self.clone();
        x.base = b;
        x
    }

    fn with_bytesformat(&self, b: BytesFormat) -> Self {
        let mut x = self.clone();
        x.bytesformat = b;
        x
    }

    fn with_strformat(&self, s: StrFormat) -> Self {
        let mut x = self.clone();
        x.strformat = s;
        x
    }

    fn with_compact(&self, c: bool) -> Self {
        let mut x = self.clone();
        x.compact = c;
        x
    }

    fn annotate<T>(&self, variant: Option<&str>, field: &MemberId, f: impl FnOnce(Self) -> T) -> T {
        Self::with(None, |annotator| {
            let annotator = match annotator.and_then(|a| a.format(variant, field)) {
                Some(Format::Block) => self.with_strformat(StrFormat::Multiline),
                Some(Format::Binary) => self.with_base(Base::Bin),
                Some(Format::Decimal) => self.with_base(Base::Dec),
                Some(Format::Hex) => self.with_base(Base::Hex),
                Some(Format::Octal) => self.with_base(Base::Oct),
                Some(Format::Compact) => self.with_compact(true),
                Some(Format::HexStr) => self.with_bytesformat(BytesFormat::HexStr),
                Some(Format::Hexdump) => self.with_bytesformat(BytesFormat::Hexdump),
                Some(Format::Xxd) => self.with_bytesformat(BytesFormat::Xxd),
                None => self.clone(),
            };
            f(annotator)
        })
    }

    fn comment(&self, variant: Option<&str>, field: &MemberId) -> Option<Document> {
        Self::with(None, |annotator| {
            annotator
                .and_then(|a| a.comment(variant, field))
                .map(|c| Document::Comment(c, CommentFormat::Standard))
        })
    }
}

impl<'s> ser::Serializer for &'s mut AnnotatedSerializer {
    type Ok = Document;
    type Error = Error;

    type SerializeSeq = SerializeSeq<'s>;
    type SerializeTuple = SerializeTuple<'s>;
    type SerializeTupleStruct = SerializeTupleStruct<'s>;
    type SerializeTupleVariant = SerializeTupleVariant<'s>;
    type SerializeMap = SerializeMap<'s>;
    type SerializeStruct = SerializeStruct<'s>;
    type SerializeStructVariant = SerializeStructVariant<'s>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Boolean(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Int(Int::new(v, self.base)))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Float(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Document::String(v.to_string(), self.strformat))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Document::String(v.to_string(), self.strformat))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        if let Some(string) = hexdump::to_string(v, self.bytesformat) {
            Ok(Document::String(
                string,
                if self.bytesformat == BytesFormat::HexStr {
                    StrFormat::Standard
                } else {
                    StrFormat::Multiline
                },
            ))
        } else {
            Ok(Document::Bytes(v.to_vec()))
        }
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        let node = self.serialize_str(variant)?;
        // TODO(serde-annotate#6): currently, placing a comment on a unit variant results in
        // ugly (json) or bad (yaml) documents.  For now, omit comments on
        // unit variants until we refactor comment emitting.
        //if let Some(c) = self.comment(Some(variant), &MemberId::Variant) {
        //    Ok(Document::Fragment(vec![c, node]))
        //} else {
        //    Ok(node)
        //}
        Ok(node)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let field = MemberId::Index(0);
        let node = self.annotate(None, &field, |mut ser| value.serialize(&mut ser))?;
        // TODO(serde-annotate#6): currently, placing a comment on a newtype structs results in
        // ugly (json) or bad (yaml) documents.  For now, omit comments on
        // unit variants until we refactor comment emitting.
        //if let Some(c) = self.comment(None, &field) {
        //    Ok(Document::Fragment(vec![c, node]))
        //} else {
        //    Ok(node)
        //}
        Ok(node)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let v = self.annotate(Some(variant), &MemberId::Variant, |mut ser| {
            let v = value.serialize(&mut ser)?;
            Ok::<_, Self::Error>(if ser.compact {
                Document::Compact(v.into())
            } else {
                v
            })
        })?;
        let mut nodes = vec![];
        if let Some(c) = self.comment(Some(variant), &MemberId::Variant) {
            nodes.push(c);
        }
        nodes.push(Document::from(variant));
        nodes.push(v);

        Ok(Document::Mapping(vec![Document::Fragment(nodes)]))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeSeq::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeTuple::new(self))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerializeTupleStruct::new(self))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeTupleVariant::new(self, variant))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap::new(self))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeStruct::new(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeStructVariant::new(self, variant))
    }
}

pub struct SerializeSeq<'s> {
    serializer: &'s mut AnnotatedSerializer,
    sequence: Vec<Document>,
}

impl<'s> SerializeSeq<'s> {
    fn new(s: &'s mut AnnotatedSerializer) -> Self {
        SerializeSeq {
            serializer: s,
            sequence: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeSeq for SerializeSeq<'s> {
    type Ok = Document;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.sequence.push(value.serialize(&mut *self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Sequence(self.sequence))
    }
}

pub struct SerializeTuple<'s> {
    serializer: &'s mut AnnotatedSerializer,
    sequence: Vec<Document>,
}

impl<'s> SerializeTuple<'s> {
    fn new(s: &'s mut AnnotatedSerializer) -> Self {
        SerializeTuple {
            serializer: s,
            sequence: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeTuple for SerializeTuple<'s> {
    type Ok = Document;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.sequence.push(value.serialize(&mut *self.serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Sequence(self.sequence))
    }
}

pub struct SerializeTupleStruct<'s> {
    serializer: &'s mut AnnotatedSerializer,
    index: u32,
    sequence: Vec<Document>,
}

impl<'s> SerializeTupleStruct<'s> {
    fn new(s: &'s mut AnnotatedSerializer) -> Self {
        SerializeTupleStruct {
            serializer: s,
            index: 0,
            sequence: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeTupleStruct for SerializeTupleStruct<'s> {
    type Ok = Document;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let field = MemberId::Index(self.index);
        let node = self
            .serializer
            .annotate(None, &field, |mut ser| value.serialize(&mut ser))?;
        if let Some(c) = self.serializer.comment(None, &field) {
            self.sequence.push(Document::Fragment(vec![c, node]));
        } else {
            self.sequence.push(node);
        }
        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Sequence(self.sequence))
    }
}

pub struct SerializeTupleVariant<'s> {
    serializer: &'s mut AnnotatedSerializer,
    variant: &'static str,
    index: u32,
    sequence: Vec<Document>,
}

impl<'s> SerializeTupleVariant<'s> {
    fn new(s: &'s mut AnnotatedSerializer, v: &'static str) -> Self {
        SerializeTupleVariant {
            serializer: s,
            variant: v,
            index: 0,
            sequence: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeTupleVariant for SerializeTupleVariant<'s> {
    type Ok = Document;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let field = MemberId::Index(self.index);
        let node = self
            .serializer
            .annotate(Some(self.variant), &field, |mut ser| {
                value.serialize(&mut ser)
            })?;
        if let Some(c) = self.serializer.comment(Some(self.variant), &field) {
            self.sequence.push(Document::Fragment(vec![c, node]));
        } else {
            self.sequence.push(node);
        }

        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let sequence = self
            .serializer
            .annotate(Some(self.variant), &MemberId::Variant, |ser| {
                if ser.compact {
                    Document::Compact(Document::Sequence(self.sequence).into())
                } else {
                    Document::Sequence(self.sequence)
                }
            });
        let mut nodes = vec![];
        if let Some(c) = self
            .serializer
            .comment(Some(self.variant), &MemberId::Variant)
        {
            nodes.push(c);
        }
        nodes.push(Document::from(self.variant));
        nodes.push(sequence);
        Ok(Document::Mapping(vec![Document::Fragment(nodes)]))
    }
}

pub struct SerializeMap<'s> {
    serializer: &'s mut AnnotatedSerializer,
    next_key: Option<Document>,
    mapping: Vec<Document>,
}

impl<'s> SerializeMap<'s> {
    fn new(s: &'s mut AnnotatedSerializer) -> Self {
        SerializeMap {
            serializer: s,
            next_key: None,
            mapping: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeMap for SerializeMap<'s> {
    type Ok = Document;
    type Error = Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Mapping(self.mapping))
    }

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.next_key = Some(key.serialize(&mut *self.serializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.next_key.take() {
            Some(key) => {
                self.mapping.push(Document::Fragment(vec![
                    key,
                    value.serialize(&mut *self.serializer)?,
                ]));
            }
            None => panic!("serialize_value called before serialize_key"),
        };
        Ok(())
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.mapping.push(Document::Fragment(vec![
            key.serialize(&mut *self.serializer)?,
            value.serialize(&mut *self.serializer)?,
        ]));
        Ok(())
    }
}

pub struct SerializeStruct<'s> {
    serializer: &'s mut AnnotatedSerializer,
    mapping: Vec<Document>,
}

impl<'s> SerializeStruct<'s> {
    fn new(s: &'s mut AnnotatedSerializer) -> Self {
        SerializeStruct {
            serializer: s,
            mapping: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeStruct for SerializeStruct<'s> {
    type Ok = Document;
    type Error = Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Document::Mapping(self.mapping))
    }

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let field = MemberId::Name(key);
        let mut nodes = vec![];
        if let Some(c) = self.serializer.comment(None, &field) {
            nodes.push(c);
        }
        nodes.push(Document::from(key));
        nodes.push(
            self.serializer
                .annotate(None, &field, |mut ser| value.serialize(&mut ser))?,
        );
        self.mapping.push(Document::Fragment(nodes));
        Ok(())
    }
}

pub struct SerializeStructVariant<'s> {
    serializer: &'s mut AnnotatedSerializer,
    variant: &'static str,
    mapping: Vec<Document>,
}

impl<'s> SerializeStructVariant<'s> {
    fn new(s: &'s mut AnnotatedSerializer, v: &'static str) -> Self {
        SerializeStructVariant {
            serializer: s,
            variant: v,
            mapping: Vec::new(),
        }
    }
}

impl<'s> ser::SerializeStructVariant for SerializeStructVariant<'s> {
    type Ok = Document;
    type Error = Error;

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mapping = self
            .serializer
            .annotate(Some(self.variant), &MemberId::Variant, |ser| {
                if ser.compact {
                    Document::Compact(Document::Mapping(self.mapping).into())
                } else {
                    Document::Mapping(self.mapping)
                }
            });
        let mut nodes = vec![];
        if let Some(c) = self
            .serializer
            .comment(Some(self.variant), &MemberId::Variant)
        {
            nodes.push(c);
        }
        nodes.push(Document::from(self.variant));
        nodes.push(mapping);
        Ok(Document::Mapping(vec![Document::Fragment(nodes)]))
    }

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        let field = MemberId::Name(key);
        let mut nodes = vec![];
        if let Some(c) = self.serializer.comment(None, &field) {
            nodes.push(c);
        }
        nodes.push(Document::from(key));
        nodes.push(
            self.serializer
                .annotate(None, &field, |mut ser| value.serialize(&mut ser))?,
        );
        self.mapping.push(Document::Fragment(nodes));
        Ok(())
    }
}
