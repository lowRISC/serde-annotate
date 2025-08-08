use crate::attr::{self, Attrs};
use proc_macro2::Span;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Ident, Index, Member, Result};

#[derive(Debug)]
pub enum Input<'a> {
    Struct(Struct<'a>),
    Enum(Enum<'a>),
}

#[derive(Debug)]
pub struct Struct<'a> {
    #[allow(unused)]
    pub attrs: Attrs<'a>,
    pub ident: Ident,
    pub fields: Vec<Field<'a>>,
}

#[derive(Debug)]
pub struct Field<'a> {
    pub attrs: Attrs<'a>,
    pub member: Member,
}

#[derive(Debug)]
pub struct Enum<'a> {
    #[allow(unused)]
    pub attrs: Attrs<'a>,
    pub ident: Ident,
    pub variants: Vec<Variant<'a>>,
}

#[derive(Debug)]
pub struct Variant<'a> {
    pub attrs: Attrs<'a>,
    pub ident: Ident,
    pub fields: Vec<Field<'a>>,
}

impl<'a> Input<'a> {
    pub fn from_syn(node: &'a DeriveInput) -> Result<Self> {
        match &node.data {
            Data::Struct(data) => Struct::from_syn(node, data).map(Input::Struct),
            Data::Enum(data) => Enum::from_syn(node, data).map(Input::Enum),
            Data::Union(_) => Err(Error::new_spanned(node, "unions are not supported")),
        }
    }
}

impl<'a> Struct<'a> {
    fn from_syn(node: &'a DeriveInput, data: &'a DataStruct) -> Result<Self> {
        let attrs = attr::get(&node.attrs)?;
        let span = Span::call_site();
        let fields = Field::multiple_from_syn(&data.fields, span)?;
        Ok(Struct {
            attrs,
            ident: node.ident.clone(),
            fields,
        })
    }
}

impl<'a> Enum<'a> {
    fn from_syn(node: &'a DeriveInput, data: &'a DataEnum) -> Result<Self> {
        let attrs = attr::get(&node.attrs)?;
        let span = Span::call_site();
        let variants = data
            .variants
            .iter()
            .map(|node| {
                let v = Variant::from_syn(node, span)?;
                Ok(v)
            })
            .collect::<Result<_>>()?;
        Ok(Enum {
            attrs,
            ident: node.ident.clone(),
            variants,
        })
    }
}

impl<'a> Field<'a> {
    fn multiple_from_syn(fields: &'a Fields, span: Span) -> Result<Vec<Self>> {
        fields
            .iter()
            .enumerate()
            .map(|(i, field)| Field::from_syn(i, field, span))
            .collect()
    }

    fn from_syn(i: usize, node: &'a syn::Field, span: Span) -> Result<Self> {
        Ok(Field {
            attrs: attr::get(&node.attrs)?,
            member: node.ident.clone().map(Member::Named).unwrap_or_else(|| {
                Member::Unnamed(Index {
                    index: i as u32,
                    span,
                })
            }),
        })
    }
}

impl<'a> Variant<'a> {
    fn from_syn(node: &'a syn::Variant, span: Span) -> Result<Self> {
        let attrs = attr::get(&node.attrs)?;
        Ok(Variant {
            attrs,
            ident: node.ident.clone(),
            fields: Field::multiple_from_syn(&node.fields, span)?,
        })
    }
}
