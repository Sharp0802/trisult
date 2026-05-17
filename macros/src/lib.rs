#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::min_ident_chars,
    clippy::missing_inline_in_public_items,
    clippy::must_use_candidate
)]
#![allow(clippy::type_complexity)]

extern crate proc_macro;

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_error::{emit_error, proc_macro_error};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Expr, ItemFn};

mod keyword {
    syn::custom_keyword!(segment);
}

#[derive(Default)]
struct TrisultArgs {
    segment: Option<(Span, Expr)>,
}

impl Parse for TrisultArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = Self::default();

        if input.is_empty() {
            return Ok(args);
        }

        let lookahead = input.lookahead1();
        if lookahead.peek(keyword::segment) {
            let span = input.parse::<keyword::segment>()?.span;
            input.parse::<syn::Token![=]>()?;
            args.segment = Some((span, input.parse()?));
        } else {
            return Err(lookahead.error());
        }

        Ok(args)
    }
}

impl TrisultArgs {
    fn prologue(&self, ident: &Ident) -> TokenStream {
        if let Some((_, segment)) = &self.segment {
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

fn identify_generic(func: &mut ItemFn, name: &str) -> Option<Ident> {
    for arg in &mut func.sig.generics.params {
        let syn::GenericParam::Type(generic) = arg else {
            continue;
        };

        let attr_idx = generic.attrs.iter().position(|a| a.path().is_ident(name));

        let Some(idx) = attr_idx else {
            continue;
        };

        generic.attrs.remove(idx);

        return Some(generic.ident.clone());
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
                $expr.__macro_tri_unpack(&mut __trisult_diags, &mut __has_errors)
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
                ($warn:expr) => { compile_error!("A `#[context]` argument must be specified to emit diagnosis implicitly"); };
            }
            macro_rules! error {
                ($err:expr, $ctx:expr) => { __impl_error!($err, $ctx) };
                ($err:expr) => { compile_error!("A `#[context]` argument must be specified to emit diagnosis implicitly"); };
            }
        }
    }
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn trisult(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut args = syn::parse_macro_input!(attr as TrisultArgs);
    let mut func = parse_macro_input!(input as ItemFn);

    let context = identify_attr(&mut func, "context");
    let kind = identify_generic(&mut func, "kind");
    let macros = quote_macros(context.as_ref());

    let return_type = match &func.sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };

    if let Some((span, segment)) = &mut args.segment {
        if context.is_none() {
            let source = segment.span().source_text().unwrap_or("<unknown>".into());
            emit_error!(
                span, "missing context argument";
                help = "A `#[context]` argument must be specified to push a stack segment";
                note = "The segment '{}' needs a context stack to push into", source;
            );
            args.segment = None;
        }
    }

    let state = if let Some(kind) = kind {
        quote! { #kind }
    } else {
        #[cfg(feature = "alloc")]
        let tmp = quote! { ::trisult::All };
        #[cfg(not(feature = "alloc"))]
        let tmp = quote! { ::trisult::Most };
        tmp
    };

    let push = context
        .as_ref()
        .map_or_else(|| quote! {}, |context| args.prologue(context));
    let pop = context
        .as_ref()
        .map_or_else(|| quote! {}, |context| args.epilogue(context));

    let prologue = quote! { #push #macros };
    let epilogue = quote! { #pop };

    let original_block = &func.block;
    let expanded_block = quote! {
        {
            use ::trisult::{AccAlloc, Accumulator, ContextStackMut};

            trait __TrisultInfer {
                type T;
                type W;
                type E;
                type C: ::trisult::CapturedContext;
            }

            impl<__T, __W, __E, __C, __A> __TrisultInfer for ::trisult::Trisult<__T, __W, __E, __C, __A>
            where
                __C: ::trisult::CapturedContext,
                __A: ::trisult::Accumulator<::trisult::Diagnosis<__W, __E>, __C>,
            {
                type T = __T;
                type W = __W;
                type E = __E;
                type C = __C;
            }

            let mut __trisult_diags = ::trisult::Diagnoses::new(
                #state ::create_state::<
                    ::trisult::Diagnosis<
                        <#return_type as __TrisultInfer>::W,
                        <#return_type as __TrisultInfer>::E
                    >,
                    <#return_type as __TrisultInfer>::C
                >()
            );

            let mut __has_errors = false;

            #prologue

            let mut __trisult_body = || -> core::option::Option<<#return_type as __TrisultInfer>::T> {
                #original_block
            };
            let __trisult_res = __trisult_body();

            #epilogue

            let __trisult_final: #return_type = if __has_errors || __trisult_res.is_none() {
                ::trisult::Trisult::Err(__trisult_diags)
            } else {
                let __warnings = __trisult_diags.unwrap_as_warnings();
                ::trisult::Trisult::Ok(::trisult::Diagnosed(__trisult_res.unwrap(), __warnings))
            };

            __trisult_final
        }
    };

    *func.block = syn::parse2(expanded_block).expect("Failed to parse expanded block");

    quote! { #func }.into()
}
