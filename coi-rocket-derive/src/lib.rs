//! Provides the `inject` proc macro for use by the [`coi-rocket`] crate.
//!
//! [`coi-rocket`]: https://docs.rs/coi-rocket

extern crate proc_macro;
use crate::{
    attr::Inject,
    ctxt::Ctxt,
    symbols::{ARC, INJECT},
};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, Error, FnArg, GenericArgument, Ident, ItemFn, Pat,
    PathArguments, Result, Type, TypePath,
};

mod attr;
mod ctxt;
mod symbols;

fn get_arc_ty(ty: &Type, type_path: &TypePath) -> Result<Type> {
    let make_arc_error = || Err(Error::new_spanned(ty, "only Arc<...> can be injected"));
    if type_path.path.leading_colon.is_some() || type_path.path.segments.len() != 1 {
        return make_arc_error();
    }
    let segment = &type_path.path.segments[0];
    if segment.ident != ARC {
        return make_arc_error();
    }
    let angle_args = match &segment.arguments {
        PathArguments::AngleBracketed(angle_args) => angle_args,
        _ => return make_arc_error(),
    };
    let args = &angle_args.args;
    if args.len() != 1 {
        return make_arc_error();
    }

    if let GenericArgument::Type(ty) = &args[0] {
        Ok(ty.clone())
    } else {
        make_arc_error()
    }
}

/// The #[inject] proc macro should only be applied to functions that will
/// be passed to [`rocket`]'s routing APIs.
///
/// [`rocket`]: https://rocket.rs
///
/// ## Examples
/// ```rust,no_run
/// #![feature(decl_macro)]
///
/// use coi::Inject;
/// use coi_rocket::inject;
/// use rocket::get;
/// use std::sync::Arc;
///
/// # trait IService : Inject {}
///
/// #[inject]
/// #[get("/path")]
/// fn get_all(#[inject] service: Arc<dyn IService>) -> String {
///     // use service here...
///     String::from("Hello, World")
/// }
/// ```
#[proc_macro_attribute]
pub fn inject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as Inject);
    let cr = attr.crate_path;

    let mut input = parse_macro_input!(input as ItemFn);
    let fn_ident = input.sig.ident.clone();
    let sig = &mut input.sig;
    let mut defs = vec![];
    let mut stmts = vec![];
    let mut ctxt = Ctxt::new();
    for arg in &mut sig.inputs {
        if let FnArg::Typed(arg) = arg {
            if arg.attrs.iter().any(|attr| attr.path() == INJECT) {
                arg.attrs.retain(|attr| attr.path() != INJECT);
                let key: Ident = if let Pat::Ident(pat_ident) = &*arg.pat {
                    let ident = &pat_ident.ident;
                    parse_quote! { #ident }
                } else {
                    ctxt.push_spanned(&*arg.pat, "patterns cannot be injected");
                    continue;
                };

                let arc_ty = &*arg.ty;
                let ty = if let Type::Path(type_path) = &*arg.ty {
                    match get_arc_ty(&arg.ty, type_path) {
                        Ok(ty) => ty,
                        Err(e) => {
                            ctxt.push_spanned(&*arg.ty, e);
                            continue;
                        }
                    }
                } else {
                    ctxt.push_spanned(&*arg.ty, "only Arc<...> can be injected");
                    continue;
                };

                let ident = format_ident!("__{}_{}_Key", fn_ident, key);
                let key_str = format!("{}", key);
                defs.push(quote! {
                    #[allow(non_camel_case_types)]
                    struct #ident;
                    impl #cr::ContainerKey<#ty> for #ident {
                        const KEY: &'static str = #key_str;
                    }
                });

                stmts.push(parse_quote!( let #cr::Injected(#key, _) = #key; ));
                *arg.ty = parse_quote!( #cr::Injected<#arc_ty, #ident> );
            }
        }
    }

    input.block.stmts = stmts.into_iter().chain(input.block.stmts).collect();

    if let Err(e) = ctxt.check() {
        let compile_errors = e.iter().map(Error::to_compile_error);
        return quote!(#( #compile_errors )*).into();
    }

    let expanded = quote! {
        #( #defs )*
        #input
    };
    TokenStream::from(expanded)
}
