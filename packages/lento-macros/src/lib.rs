use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Fields, FnArg, Ident, ImplItem, ItemFn, ItemImpl, ItemStruct, Visibility};
use syn::__private::TokenStream2;
use syn::token::{Async};

#[proc_macro_attribute]
pub fn mrc_object(_attr: TokenStream, struct_def: TokenStream) -> TokenStream {
    let struct_def = parse_macro_input!(struct_def as ItemStruct);
    let weak_name = format_ident!("{}Weak", struct_def.ident);
    let struct_name = format_ident!("{}Data", struct_def.ident);
    let ref_name = struct_def.ident;
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

            pub fn from_inner(inner: lento::mrc::Mrc<#struct_name>) -> Self {
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
            pub fn upgrade(&self) -> Result<#ref_name, lento::mrc::UpgradeError> {
                let inner = self.inner.upgrade()?;
                Ok(
                     #ref_name {
                        inner
                    }
                )
            }

            pub fn upgrade_mut<R, F: FnOnce(&mut #ref_name) -> R>(&self, callback: F) -> Option<R> {
                if let Ok(f) = self.inner.upgrade() {
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
                #ref_name::from_inner(inner)
            }
        }

    };
    expanded.into()
}


#[proc_macro_attribute]
pub fn event(_attr: TokenStream, struct_def: TokenStream) -> TokenStream {
    create_event(_attr, struct_def, quote! {lento::element::ElementWeak})
}

#[proc_macro_attribute]
pub fn frame_event(_attr: TokenStream, struct_def: TokenStream) -> TokenStream {
    create_event(_attr, struct_def, quote! {lento::frame::FrameWeak})
}

fn create_event(_attr: TokenStream, struct_def: TokenStream, target_type: TokenStream2) -> TokenStream {
    let struct_def = parse_macro_input!(struct_def as ItemStruct);
    let listener_name = format_ident!("{}Listener", struct_def.ident);
    let event_name = struct_def.ident;
    let fields = struct_def.fields;

    let fields_ts = match fields {
        Fields::Named(nf) => { quote! {#nf} }
        Fields::Unnamed(uf) => { quote! {#uf;} }
        Fields::Unit => {quote! {#fields;} }
    };

    let expanded = quote! {

        pub struct #listener_name(Box<dyn FnMut(&mut #event_name, &mut lento::base::EventContext<#target_type>)>);

        impl #listener_name {
            pub fn new<F: FnMut(&mut #event_name, &mut lento::base::EventContext<#target_type>) + 'static>(f: F) -> Self {
                Self(Box::new(f))
            }
        }

        impl lento::base::EventListener<#event_name, #target_type> for #listener_name {
            fn handle_event(&mut self, event: &mut #event_name, ctx: &mut lento::base::EventContext<#target_type>) {
                (self.0)(event, ctx)
            }
        }

        impl lento::js::FromJsValue for #listener_name {
            fn from_js_value(value: JsValue) -> Result<Self, quick_js::ValueError> {
                let listener = Self::new(move |e, ctx| {
                    let target = ctx.target.clone();
                    use lento::js::ToJsValue;
                    if let Ok(d) = target.to_js_value() {
                        use lento::js::js_value_util::ToJsValue;
                        use lento::js::js_value_util::SerializeToJsValue;
                        if let Ok(e) = e.to_js_value() {
                            let callback_result = value.call_as_function(vec![e, d]);
                            if let Ok(cb_result) = callback_result {
                                if let Ok(res) = lento::js::js_value_util::EventResult::from_js_value(cb_result) {
                                    if res.propagation_cancelled {
                                        ctx.propagation_cancelled = true;
                                    }
                                    if res.prevent_default {
                                        ctx.prevent_default = true;
                                    }
                                }
                            }
                        } else {
                            println!("invalid event");
                        }
                    } else {
                        println!("invalid event");
                    }
                });
                Ok(listener)
            }
        }

        #[derive(serde::Serialize)]
        pub struct #event_name
            #fields_ts

        impl lento::element::ViewEvent for #event_name {
            fn allow_bubbles(&self) -> bool {
                false
            }
        }

    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn js_methods(_attr: TokenStream, impl_item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(impl_item as ItemImpl);
    // item.self_ty.into_token_stream();
    let ItemImpl {
        attrs,
        impl_token,
        generics,
        self_ty,
        mut items,
        ..
    } = item;

    let mut api_bridges = Vec::new();
    let mut api_create_expr_list = Vec::new();
    let type_name_str = self_ty.clone().into_token_stream().to_string();
    let type_name_ident = format_ident!("{}", type_name_str);


    for item in &mut items {
        match item {
            ImplItem::Fn(item) => {
                item.attrs.retain(|it| {
                    if !it.path().is_ident("js_func") {
                        return true;
                    }

                    let vis = item.vis.clone();

                    let api_name_ident = format_ident!("{}_{}", type_name_str, item.sig.ident);

                    let args_count = item.sig.inputs.len();
                    let args = item.sig.inputs.iter().map(|it| it.clone()).collect::<Vec<_>>();

                    let bridge_body = build_bridge_body(
                        args,
                        item.sig.asyncness,
                        type_name_ident.clone(),
                        item.sig.ident.clone()
                    );

                    let bridge = build_bridge_struct(
                        vis,
                        api_name_ident.clone(),
                        args_count,
                        bridge_body,
                    );

                    api_bridges.push(bridge);
                    api_create_expr_list.push(quote! {
                        #api_name_ident::new()
                    });
                    false
                });
            }
            _ => {}
        }
    }
    let q = quote! {
        #(#attrs)*
        #impl_token #generics #self_ty {
            #(#items)*

            pub fn create_js_apis() -> Vec<Box<dyn lento::js::JsFunc + std::panic::RefUnwindSafe + 'static>> {
                vec![#(Box::new(#api_create_expr_list), )*]
            }

        }

        #(#api_bridges)*
    };

    q.into()
}

fn build_bridge_struct(vis: Visibility, func_name: Ident, args_count: usize, bridge_body: TokenStream2) -> TokenStream2 {
    let func_name_str = func_name.to_string();
    quote! {
        #[doc(hidden)]
        #[allow(nonstandard_style)]
        #vis struct #func_name  {}

        impl #func_name {
            pub fn new() -> Self {
                Self {}
            }
        }

        impl lento::js::JsFunc for #func_name {

            fn name(&self) -> &str {
                #func_name_str
            }

            fn args_count(&self) -> usize {
                #args_count
            }

            fn call(&self, js_context: &mut lento::mrc::Mrc<lento::js::JsContext>, args: Vec<lento::js::JsValue>) -> Result<lento::js::JsValue, lento::js::JsCallError> {
                #bridge_body
            }
        }
    }
}

fn build_bridge_body(func_inputs: Vec<FnArg>, asyncness: Option<Async>, struct_name: Ident, func_name: Ident) -> TokenStream2 {
    let mut receiver = None;
    let mut params = Vec::new();
    func_inputs.iter().for_each(|i| {
        match i {
            FnArg::Receiver(r) => receiver = Some(r.ty.clone()),
            FnArg::Typed(ref val) => {
                params.push(val.ty.clone())
            }
        }
    });
    let mut param_expand_stmts = Vec::new();
    let mut param_list = Vec::new();
    let mut idx = if receiver.is_some() { 1usize } else { 0usize };
    for p in params {
        let p_name = format_ident!("_p{}", idx);
        param_expand_stmts.push(quote! {
            let #p_name = <#p as lento::js::FromJsValue>::from_js_value(args.get(#idx).unwrap().clone())?;
        });
        param_list.push(p_name);
        idx += 1;
    }

    // let return_type = func.sig.output;

    let call_stmt = if asyncness.is_none() {
        if receiver.is_some() {
            quote! {
                let mut inst = <#struct_name as lento::js::FromJsValue>::from_js_value(args.get(0).unwrap().clone())?;
                let r = inst.#func_name( #(#param_list, )* );
            }
        } else {
            quote! {
                let r = #struct_name::#func_name( #(#param_list, )* );
            }
        }
    } else {
        if receiver.is_some() {
            quote! {
                let inst = <#struct_name as lento::js::FromJsValue>::from_js_value(args.get(0).unwrap().clone())?;
                let r = js_context.create_async_task2(async move {
                    inst.#func_name( #(#param_list, )* ).await
                });
            }
        } else {
            quote! {
                let r = js_context.create_async_task2(async move {
                    #struct_name::#func_name( #(#param_list, )* ).await
                });
            }
        }
    };
    let result = quote! {
        use lento::js::FromJsValue;
        use lento::js::ToJsValue;
        use lento::js::ToJsCallResult;
        #(#param_expand_stmts)*
        #call_stmt
        r.to_js_call_result()
    };
    result
}

#[proc_macro_attribute]
pub fn js_func(_attr: TokenStream, func: TokenStream) -> TokenStream {
    let func = parse_macro_input!(func as ItemFn);
    let vis = func.vis;
    let func_name = &func.sig.ident;
    let asyncness = func.sig.asyncness;
    let func_inputs = func.sig.inputs;
    let func_block = func.block;

    let args_count = func_inputs.len();
    let args = func_inputs.iter().map(|it| it.clone()).collect::<Vec<_>>();
    let bridge_body = build_bridge_body(
        args,
        asyncness,
        format_ident!("Self"),
        func_name.clone()
    );

    let return_type = func.sig.output;

    let bridge = build_bridge_struct(vis, func_name.clone(), args_count, bridge_body);

    let expanded = quote! {

        #bridge

        impl #func_name {

            #asyncness fn #func_name(#func_inputs) #return_type #func_block

        }

    };
    expanded.into()
}