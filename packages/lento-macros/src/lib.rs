use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemFn, ItemStruct};

#[proc_macro_attribute]
pub fn mrc_object(_attr: TokenStream, struct_def: TokenStream) -> TokenStream {
    let struct_def = parse_macro_input!(struct_def as ItemStruct);
    let weak_name = format_ident!("{}Weak", struct_def.ident);
    let ref_name = format_ident!("{}Ref", struct_def.ident);
    let struct_name = struct_def.ident;
    let fields = struct_def.fields;

    let expanded = quote! {

        #[derive(Clone, PartialEq)]
        pub struct #ref_name {
            inner: lento::mrc::Mrc<#struct_name>,
        }

        impl std::ops::Deref for #ref_name {
            type Target = lento::mrc::Mrc<#struct_name>;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl std::ops::DerefMut for #ref_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }

        impl #ref_name {

            pub fn new(inner: lento::mrc::Mrc<#struct_name>) -> Self {
                Self { inner }
            }

            pub fn as_weak(&self) -> #weak_name {
                #weak_name {
                    inner: self.inner.as_weak(),
                }
            }
        }

        #[derive(Clone)]
        pub struct #weak_name {
            inner: lento::mrc::MrcWeak<#struct_name>,
        }

        impl #weak_name {
            pub fn upgrade(&self) -> Option<#ref_name> {
                if let Some(f) = self.inner.upgrade() {
                    let mut inst = #ref_name {
                        inner: f
                    };
                    Some(inst)
                } else {
                    None
                }
            }

            pub fn upgrade_mut<R, F: FnOnce(&mut #ref_name) -> R>(&self, callback: F) -> Option<R> {
                if let Some(f) = self.inner.upgrade() {
                    let mut inst = #ref_name {
                        inner: f
                    };
                    Some(callback(&mut inst))
                } else {
                    None
                }
            }

        }

        pub struct #struct_name
            #fields


        impl #struct_name {
            pub fn to_ref(self) -> #ref_name {
                let inner = lento::mrc::Mrc::new(self);
                #ref_name::new(inner)
            }
        }

    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn js_func(_attr: TokenStream, func: TokenStream) -> TokenStream {
    let func = parse_macro_input!(func as ItemFn);
    let vis = func.vis;
    let func_name = &func.sig.ident;
    let asyncness = func.sig.asyncness;
    let func_name_str = func_name.to_string();
    let func_inputs = func.sig.inputs;
    let func_block = func.block;
    let params: Vec<_> = func_inputs.iter().map(|i| {
        match i {
            FnArg::Receiver(_) => unreachable!(),
            FnArg::Typed(ref val) => {
                &val.ty
            }
        }
    }).collect();
    let mut param_expand_stmts = Vec::new();
    let mut param_list = Vec::new();
    let mut idx = 0usize;
    for p in params {
        let p_name = format_ident!("_p{}", idx);
        param_expand_stmts.push(quote! {
            let #p_name = #p::from_js_value(args.get(#idx).unwrap().clone())?;
        });
        param_list.push(p_name);
        idx += 1;
    }

    let return_type = func.sig.output;

    let call_stmt = if asyncness.is_none() {
        quote! {
            let r = Self::#func_name( #(#param_list, )* );
        }
    } else {
        quote! {
            let r = js_context.create_async_task2(async move {
                Self::#func_name( #(#param_list, )* ).await
            });
        }
    };

    let expanded = quote! {

        #[doc(hidden)]
        #[allow(nonstandard_style)]
        #vis struct #func_name  {}

        impl #func_name {

            #asyncness fn #func_name(#func_inputs) #return_type #func_block

            pub fn new() -> Self {
                Self {}
            }

        }

        impl lento::js::JsFunc for #func_name {
            fn name(&self) -> &str {
                #func_name_str
            }

            fn args_count(&self) -> usize {
                #idx
            }

            fn call(&self, js_context: &mut lento::mrc::Mrc<lento::js::JsContext>, args: Vec<lento::js::JsValue>) -> Result<lento::js::JsValue, lento::js::JsCallError> {
                use lento::js::FromJsValue;
                use lento::js::ToJsValue;
                use lento::js::ToJsCallResult;
                #(#param_expand_stmts)*
                #call_stmt
                r.to_js_call_result()
            }
        }

    };
    expanded.into()
}