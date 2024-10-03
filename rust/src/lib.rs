extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

#[proc_macro_derive(FromStrImpl)]
pub fn from_str_impl_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let name = &input.ident;

    let expanded = quote! {
        impl #name {
            pub fn from_str<'de, D>(deserializer: D) -> Result<i32, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: &str = serde::Deserialize::deserialize(deserializer)?;
                Self::from_str_name(s)
                    .map(|v| v as i32)
                    .ok_or(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Option,
                        &"a valid enum value",
                    ))
            }
        }
    };

    TokenStream::from(expanded)
}
