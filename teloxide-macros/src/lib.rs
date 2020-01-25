mod attr;
mod command;
mod enum_attributes;
mod rename_rules;

extern crate proc_macro;
extern crate syn;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{DeriveInput, parse_macro_input};
use crate::command::Command;
use crate::attr::{Attr, VecAttrs};
use crate::enum_attributes::CommandEnum;
use crate::rename_rules::rename_by_rule;

macro_rules! get_or_return {
    ($($some:tt)*) => {
        match $($some)* {
            Ok(elem) => elem,
            Err(e) => return e
        };
    }
}

#[proc_macro_derive(BotCommand, attributes(command))]
pub fn derive_telegram_command_enum(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);

    let data_enum: &syn::DataEnum = get_or_return!(get_enum_data(&input));

    let enum_attrs: Vec<Attr> = get_or_return!(parse_attributes(&input.attrs));

    let command_enum = match CommandEnum::try_from(enum_attrs.as_slice()) {
        Ok(command_enum) => command_enum,
        Err(e) => return compile_error(e),
    };

    let variants: Vec<&syn::Variant> = data_enum.variants.iter().map(|attr| attr).collect();

    let mut variant_infos = vec![];
    for variant in variants.iter() {
        let mut attrs = Vec::new();
        for attr in &variant.attrs {
            match attr.parse_args::<VecAttrs>() {
                Ok(mut attrs_) => {
                    attrs.append(attrs_.data.as_mut());
                },
                Err(e) => {
                    return compile_error(e.to_compile_error());
                },
            }
        }
        match Command::try_from(attrs.as_slice(), &variant.ident.to_string()) {
            Ok(command) => variant_infos.push(command),
            Err(e) => return compile_error(e),
        }
    }

    let variant_ident = variants.iter().map(|variant| &variant.ident);
    let variant_name = variant_infos.iter().map(|info| {
        if info.renamed {
            info.name.clone()
        }
        else if let Some(rename_rule) = &command_enum.rename_rule {
            rename_by_rule(&info.name, rename_rule)
        }
        else {
            info.name.clone()
        }
    });
    let variant_prefixes = variant_infos.iter().map(|info| {
            if let Some(prefix) = &info.prefix {
                prefix
            }
            else if let Some(prefix) = &command_enum.prefix {
                prefix
            }
            else {
                "/"
            }
    });
    let variant_str1 = variant_prefixes.zip(variant_name).map(|(prefix, command)| prefix.to_string() + command.as_str());
    let variant_str2 = variant_str1.clone();
    let variant_description = variant_infos.iter().map(|info| info.description.as_ref().map(String::as_str).unwrap_or(""));

    let ident = &input.ident;

    let expanded = quote! {
        impl BotCommand for #ident {
            fn try_from(value: &str) -> Option<Self> {
                match value {
                    #(
                        #variant_str1 => Some(Self::#variant_ident),
                    )*
                    _ => None
                }
            }
            fn descriptions() -> String {
                std::concat!(#(#variant_str2, " - ", #variant_description, '\n'),*).to_string()
            }
        }
    };
    //for debug
    //println!("{}", &expanded.to_string());
    let tokens = TokenStream::from(expanded);
    tokens
}

fn get_enum_data(input: &DeriveInput) -> Result<&syn::DataEnum, TokenStream> {
    match &input.data {
        syn::Data::Enum(data) => Ok(data),
        _ => Err(compile_error("TelegramBotCommand allowed only for enums"))
    }
}

fn parse_attributes(input: &Vec<syn::Attribute>) -> Result<Vec<Attr>, TokenStream> {
    let mut enum_attrs = Vec::new();
    for attr in input.iter() {
        match attr.parse_args::<VecAttrs>() {
            Ok(mut attrs_) => {
                enum_attrs.append(attrs_.data.as_mut());
            },
            Err(e) => {
                return Err(compile_error(e.to_compile_error()));
            },
        }
    }
    Ok(enum_attrs)
}

fn compile_error<T>(data: T) -> TokenStream
where
    T: ToTokens
{
    TokenStream::from(quote! { compile_error!(#data) })
}
