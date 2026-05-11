extern crate proc_macro;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Expr, ItemFn};

mod keyword {
    syn::custom_keyword!(segment);
}

#[derive(Default)]
struct TrisultArgs {
    segment: Option<Expr>,
}

impl Parse for TrisultArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = TrisultArgs::default();

        if input.is_empty() {
            return Ok(args);
        }

        let lookahead = input.lookahead1();
        if lookahead.peek(keyword::segment) {
            input.parse::<keyword::segment>()?;
            input.parse::<syn::Token![=]>()?;
            args.segment = Some(input.parse()?);
        } else {
            return Err(lookahead.error());
        }

        Ok(args)
    }
}

impl TrisultArgs {
    fn prologue(&self, ident: &Ident) -> TokenStream {
        if let Some(segment) = &self.segment {
            quote! { #ident.push( #segment.into() ); }
        } else {
            quote! {}
        }
    }

    fn epilogue(&self, ident: &Ident) -> TokenStream {
        if self.segment.is_some() {
            quote! { #ident.pop(); }
        } else {
            quote! {}
        }
    }
}

fn identify_attr(func: &mut ItemFn, name: &str) -> Option<Ident> {
    for arg in &mut func.sig.inputs {
        let syn::FnArg::Typed(pat_type) = arg else {
            continue;
        };

        let attr_idx = pat_type.attrs.iter().position(|a| a.path().is_ident(name));

        let Some(idx) = attr_idx else {
            continue;
        };

        pat_type.attrs.remove(idx);
        if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
            return Some(pat_ident.ident.clone());
        }

        break;
    }

    None
}

fn quote_macros(context: Option<&Ident>) -> TokenStream {
    let defaults = quote! {
        macro_rules! __impl_warn {
            ($warn:expr, $ctx:expr) => {
                __trisult_diags.push(::trisult::Contextual::new(
                    $ctx,
                    ::trisult::Diagnosis::Warning($warn)
                ));
            };
        }
        macro_rules! __impl_error {
            ($err:expr, $ctx:expr) => {
                __has_errors = true;
                __trisult_diags.push(::trisult::Contextual::new(
                    $ctx,
                    ::trisult::Diagnosis::Error($err)
                ));
            };
        }

        macro_rules! tri {
            ($expr:expr) => {
                match $expr {
                    ::trisult::Trisult::Ok(::trisult::Diagnosed(val, diags)) => {
                        __trisult_diags.append(diags.map(::trisult::Diagnosis::Warning));
                        Some(val)
                    }
                    ::trisult::Trisult::Err(diags) => {
                        __has_errors = true;
                        __trisult_diags.append(diags);
                        None
                    }
                }
            };
        }
    };

    if let Some(ident) = context {
        quote! {
            #defaults

            macro_rules! warn {
                ($warn:expr, $ctx:expr) => { __impl_warn!($warn, $ctx) };
                ($warn:expr) => { warn!($warn, #ident.capture()) };
            }
            macro_rules! error {
                ($err:expr, $ctx:expr) => { __impl_error!($err, $ctx) };
                ($err:expr) => { error!($err, #ident.capture()) };
            }
        }
    } else {
        quote! {
            #defaults

            macro_rules! warn {
                ($warn:expr, $ctx:expr) => { __impl_warn!($warn, $ctx) };
            }
            macro_rules! error {
                ($err:expr, $ctx:expr) => { __impl_error!($err, $ctx) };
            }
        }
    }
}

#[proc_macro_attribute]
pub fn trisult(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(attr as TrisultArgs);
    let mut func = parse_macro_input!(input as ItemFn);

    let context = identify_attr(&mut func, "context");
    let kind = identify_attr(&mut func, "kind");
    let macros = quote_macros(context.as_ref());

    if args.segment.is_some() && context.is_none() {
        let err = syn::Error::new_spanned(
            &func.sig.ident,
            "A #[context] argument must be specified to push a stack segment.",
        )
        .to_compile_error();
        return quote! { #err #func }.into();
    }

    let kind = if let Some(kind) = kind {
        quote! { #kind }
    } else {
        quote! { ::trisult::AccumulatorKind::All }
    };

    let push = context
        .as_ref()
        .map(|context| args.prologue(context))
        .unwrap_or_else(|| quote! {});
    let pop = context
        .as_ref()
        .map(|context| args.epilogue(context))
        .unwrap_or_else(|| quote! {});

    let prologue = quote! { #push #macros };
    let epilogue = quote! { #pop };

    let original_block = &func.block;
    let expanded_block = quote! {
        {
            use ::trisult::ContextStackMut;

            let mut __trisult_diags = ::trisult::Diagnoses::new( #kind );
            let mut __has_errors = false;

            #prologue

            let mut __trisult_body = || {
                #original_block
            };
            let __trisult_res = __trisult_body();

            #epilogue

            if __has_errors || __trisult_res.is_none() {
                ::trisult::Trisult::Err(__trisult_diags)
            } else {
                let __warnings = __trisult_diags.unwrap_as_warnings();
                ::trisult::Trisult::Ok(::trisult::Diagnosed(__trisult_res.unwrap(), __warnings))
            }
        }
    };

    *func.block = syn::parse2(expanded_block).expect("Failed to parse expanded block");

    quote! { #func }.into()
}
