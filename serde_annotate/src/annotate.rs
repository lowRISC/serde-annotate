use std::fmt;

use crate::{AnnotatedSerializer, Document, Error};

/// Specifies the formatting options to use when serializing.
pub enum Format {
    /// Format a string in block/multiline style.
    Block,
    /// Format an integer as binary.
    Binary,
    /// Format an integer as decimal.
    Decimal,
    /// Format an integer as hexadecimal.
    Hex,
    /// Format an integer as octal.
    Octal,
    /// Format an aggregate in compact mode.
    Compact,
    /// Format a bytes object as a hex string.
    HexStr,
    /// Format a bytes object as hexdump (e.g. `hexdump -vC <file>`).
    Hexdump,
    /// Format a bytes object as xxd (e.g. `xxd <file>`).
    Xxd,
}

/// Identifies a field or variant member of a struct/enum.
pub enum MemberId<'a> {
    Name(&'a str),
    Index(u32),
    Variant,
}

/// Trait indicating a type can be serialized by `AnnotatedSerialize`.
///
/// This is specialized version of `Serialize` trait that operates only on `AnnotatedSerializer`,
/// so it is dyn-safe.
pub trait AnnotateSerialize {
    fn annotated_serialize(&self, serializer: &mut AnnotatedSerializer) -> Result<Document, Error>;
}

impl<T: serde::Serialize + ?Sized> AnnotateSerialize for T {
    fn annotated_serialize(&self, serializer: &mut AnnotatedSerializer) -> Result<Document, Error> {
        self.serialize(serializer)
    }
}

impl serde::Serialize for dyn AnnotateSerialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        AnnotatedSerializer::specialize(serializer, |serializer| {
            self.annotated_serialize(serializer)
        })
    }
}

impl fmt::Debug for dyn AnnotateSerialize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dyn AnnotateSerialize({:p})", self)
    }
}

/// Trait implemented on structs to inform the serializer about formatting
/// options and comments.
pub trait Annotate: AnnotateSerialize {
    fn format(&self, variant: Option<&str>, field: &MemberId) -> Option<Format>;
    fn comment(&self, variant: Option<&str>, field: &MemberId) -> Option<String>;
}
