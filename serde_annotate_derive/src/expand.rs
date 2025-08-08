use crate::ast::{Enum, Field, Input, Struct, Variant};
use crate::attr::{Attrs, Comment, Format};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Index, Member, Result};

pub fn derive(mut node: DeriveInput) -> Result<TokenStream> {
    let input = Input::from_syn(&node)?;

    let annotate_imp = match input {
        Input::Struct(input) => impl_struct(input),
        Input::Enum(input) => impl_enum(input),
    };

    // In addition to the `Annotate` implementation, we also want to generate a
    // `serde::Serialize` implementation.
    //
    // We use `serde`'s `remote` attribute to put the derive implementation on a helper
    // type so we can invoke at will.

    let name = node.ident;
    let helper = syn::Ident::new(&format!("{}Helper", name), name.span());

    // Filter out all `#[annotate]` attributes from the derive inputs.
    match &mut node.data {
        syn::Data::Struct(data_struct) => {
            for f in data_struct.fields.iter_mut() {
                f.attrs.retain(|x| !x.path().is_ident("annotate"));
            }
        }
        syn::Data::Enum(data_enum) => {
            for v in data_enum.variants.iter_mut() {
                v.attrs.retain(|x| !x.path().is_ident("annotate"));
                for f in v.fields.iter_mut() {
                    f.attrs.retain(|x| !x.path().is_ident("annotate"));
                }
            }
        }
        syn::Data::Union(_) => unreachable!(),
    }
    node.ident = helper.clone();

    for attr in node.attrs.iter() {
        if attr.path().is_ident("serde") {
            return Err(syn::Error::new_spanned(attr, "use of `#[serde]` on type together with `#[derive(Annotate)]` is not yet supported"));
        }
    }

    let name_str = syn::LitStr::new(&name.to_string(), name.span());
    Ok(quote! {
        const _: () = {
            #annotate_imp

            #[derive(::serde::Serialize)]
            #[serde(remote = #name_str, rename = #name_str)]
            #node

            impl ::serde::Serialize for #name {
                fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer
                {
                    ::serde_annotate::AnnotatedSerializer::try_specialize(
                        serializer,
                        |serializer| #helper::serialize(self, &mut serializer.with_annotate(self)),
                        |serializer| #helper::serialize(self, serializer)
                    )
                }
            }
        };
    })
}

fn impl_format(a: &Attrs) -> TokenStream {
    match &a.format {
        Format::None => quote! { None },
        Format::Block => quote! { Some(::serde_annotate::annotate::Format::Block) },
        Format::Binary => quote! { Some(::serde_annotate::annotate::Format::Binary) },
        Format::Decimal => quote! { Some(::serde_annotate::annotate::Format::Decimal) },
        Format::Hex => quote! { Some(::serde_annotate::annotate::Format::Hex) },
        Format::Octal => quote! { Some(::serde_annotate::annotate::Format::Octal) },
        Format::Compact => quote! { Some(::serde_annotate::annotate::Format::Compact) },
        Format::HexStr => quote! { Some(::serde_annotate::annotate::Format::HexStr) },
        Format::Hexdump => quote! { Some(::serde_annotate::annotate::Format::Hexdump) },
        Format::Xxd => quote! { Some(::serde_annotate::annotate::Format::Xxd) },
    }
}

fn impl_field_format(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let format = impl_format(&f.attrs);
            match &f.member {
                Member::Named(id) => {
                    let id = id.to_string();
                    quote! { ::serde_annotate::annotate::MemberId::Name(#id) => #format }
                }
                Member::Unnamed(Index { index: i, .. }) => {
                    quote! { ::serde_annotate::annotate::MemberId::Index(#i) => #format }
                }
            }
        })
        .collect::<Vec<_>>()
}

fn impl_comment(a: &Attrs) -> TokenStream {
    match &a.comment {
        Comment::None => quote! { None },
        Comment::Static(s) => quote! {
            Some(#s.to_string())
        },
        Comment::Field(id) => quote! {
            Some(self.#id.to_string())
        },
        Comment::Function(id) => quote! {
            self.#id()
        },
    }
}

fn impl_field_comment(fields: &[Field]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let comment = impl_comment(&f.attrs);
            match &f.member {
                Member::Named(id) => {
                    let id = id.to_string();
                    quote! { ::serde_annotate::annotate::MemberId::Name(#id) => #comment }
                }
                Member::Unnamed(Index { index: i, .. }) => {
                    quote! { ::serde_annotate::annotate::MemberId::Index(#i) => #comment }
                }
            }
        })
        .collect::<Vec<_>>()
}

fn impl_variants(variants: &[Variant]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let formats = variants
        .iter()
        .map(|v| {
            let variant = v.ident.to_string();
            let formats = impl_field_format(&v.fields);
            let vformat = impl_format(&v.attrs);
            quote! {
                #variant => match field {
                    ::serde_annotate::annotate::MemberId::Variant => #vformat,
                    #(#formats,)*
                    _ => None,
                }
            }
        })
        .collect::<Vec<_>>();
    let comments = variants
        .iter()
        .map(|v| {
            let variant = v.ident.to_string();
            let comments = impl_field_comment(&v.fields);
            let vcomment = impl_comment(&v.attrs);
            quote! {
                #variant => match field {
                    ::serde_annotate::annotate::MemberId::Variant => #vcomment,
                    #(#comments,)*
                    _ => None,
                }
            }
        })
        .collect::<Vec<_>>();

    (formats, comments)
}

fn impl_struct(input: Struct) -> TokenStream {
    let formats = impl_field_format(&input.fields);
    let comments = impl_field_comment(&input.fields);
    let name = &input.ident;
    quote! {
        impl ::serde_annotate::annotate::Annotate for #name {
            fn format(&self, _variant: Option<&str>, field: &::serde_annotate::annotate::MemberId) -> Option<::serde_annotate::annotate::Format> {
                match field {
                    #(#formats,)*
                    _ => None,
                }
            }
            fn comment(&self, _variant: Option<&str>, field: &::serde_annotate::annotate::MemberId) -> Option<String> {
                match field {
                    #(#comments,)*
                    _ => None,
                }
            }
        }
    }
}

fn impl_enum(input: Enum) -> TokenStream {
    let (formats, comments) = impl_variants(&input.variants);
    let name = &input.ident;
    quote! {
        impl ::serde_annotate::annotate::Annotate for #name {
            fn format(&self, variant: Option<&str>, field: &::serde_annotate::annotate::MemberId) -> Option<::serde_annotate::annotate::Format> {
                let variant = variant?;
                match variant {
                    #(#formats,)*
                    _ => None,
                }
            }
            fn comment(&self, variant: Option<&str>, field: &::serde_annotate::annotate::MemberId) -> Option<String> {
                let variant = variant?;
                match variant {
                    #(#comments,)*
                    _ => None,
                }
            }
        }
    }
}
