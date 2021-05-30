use derive_syn_parse::Parse;
use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::str::FromStr;
use syn::parse::{Parse, ParseBuffer, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, Signature, *};

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    //println!("attr: \"{}\"", attr.to_string());
    //println!("item: \"{}\"", item.to_string());

    let service = parse_macro_input!(attr as Service);
    let mut service_trait = parse_macro_input!(item as syn::ItemTrait);

    for entry in &service.entries {
        //println!("Entry");
        //println!("  ident:{}", entry.ident.to_string());
        let entry_name = entry.ident.to_string();
        if (entry_name.as_str() != "fields")
            && (entry_name.as_str() != "events")
            && (entry_name.as_str() != "method_ids")
        {
            return quote_spanned! {
                entry.ident.span() =>
                compile_error!("unexpected entry. Expected `fields`, `events` or `method_ids`");
            }
            .into();
        }
    }

    add_super_trait(&mut service_trait);

    let item_structs = create_internal_marshalling_structs(&service_trait);

    let new_methods = get_methods(&service);
    for m in new_methods {
        service_trait.items.push(syn::TraitItem::Method(m));
    }

    let mut token_stream = TokenStream2::new();
    service_trait.to_tokens(&mut token_stream);

    for item in item_structs {
        item.to_tokens(&mut token_stream);
    }

    let dispatch_handler = create_dispatch_handler(&service_trait.ident, &service, &service_trait);

    let dispatch_handler_with_use = quote! {
        use someip::*;
        use bincode::{deserialize, serialize};
        use bytes::Bytes;
        #dispatch_handler
    };
    dispatch_handler_with_use.to_tokens(&mut token_stream);

    let proxy_tokens = create_proxy(&service, &service_trait);
    proxy_tokens.to_tokens(&mut token_stream);

    println!("GENERATED:{}", &token_stream.to_string());
    token_stream.into()
}

fn create_proxy(service: &Service, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let struct_name = format_ident!("{}Proxy", item_trait.ident);

    let mut tokens_struct = quote! {
        pub struct #struct_name {
            client : std::sync::Arc<std::sync::RwLock<someip::client::Client>>,
        }

        impl #struct_name {
            pub fn new(service_id: u16, client_id: u16, config: someip::Configuration) -> Self {
                #struct_name {
                    client : std::sync::Arc::new(std::sync::RwLock::new(someip::client::Client::new(service_id, client_id, config))),
                }
            }
            pub async fn run(this: std::sync::Arc<std::sync::RwLock<Box<Self>>>, to: std::net::SocketAddr) -> Result<(), io::Error> {
                let client = {
                    let this = this.read().unwrap();
                    let client = this.client.clone();
                    client
                };
                someip::client::Client::run_static(client, to).await
            }
        }
    };

    let id_idents = get_method_ids(service);

    let methods: Vec<TokenStream2> = id_idents
        .iter()
        .map(|(i, ident)| get_client_method_by_ident(*i as u16, ident, item_trait))
        .collect();

    let mut tokens_impl = quote! {
        impl #struct_name {
            #(#methods)*
        }
    };

    tokens_struct.extend(tokens_impl);

    tokens_struct.into()
}

fn get_result_types(p: &syn::TypePath) -> Option<(syn::GenericArgument, syn::GenericArgument)> {
    if let Some(first_segment) = p.path.segments.iter().take(1).next() {
        if first_segment.ident != "Result" {
            None
        } else {
            if let syn::PathArguments::AngleBracketed(angle_bracketed_args) =
                &first_segment.arguments
            {
                let mut iter = angle_bracketed_args.args.iter();
                if let Some(success) = iter.next() {
                    if let Some(failure) = iter.next() {
                        Some((success.clone(), failure.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    } else {
        None
    }
}

fn method_has_return_type(method: &syn::TraitItemMethod) -> bool {
    match method.sig.output {
        ReturnType::Default => false,
        _ => true,
    }
}

/// Generate the code for a single RPC method in the trait
fn get_client_method_by_ident(id: u16, ident: &Ident, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let method = find_method_by_ident(ident, item_trait).expect("Expected to find method");
    let params = &method.sig.inputs;

    let maybe_return_types = match method.sig.output.clone() {
        ReturnType::Type(_a, t) => {
            if let syn::Type::Path(p) = *t.clone() {
                if let Some((success_type, failure_type)) = get_result_types(&p) {
                    println!("Found return type : {:?} {:?}", success_type, failure_type);
                    Some((success_type, failure_type))
                } else {
                    return quote_spanned! {
                        t.span() =>
                        compile_error!("Return type for SOME/IP Methods should be empty or Result");
                    }
                    .into();
                }
            } else {
                return quote_spanned! {
                    t.span() =>
                    compile_error!("Unsupported Return type ");
                }
                .into();
            }
        }
        ReturnType::Default => None,
    };
    let return_tokens = if let Some((success, failure)) = &maybe_return_types {
        quote! {
            -> Result<#success, MethodError<#failure>>
        }
    } else {
        // No return type
        quote! {-> Result<(), ()>}
    };

    let struct_members: Vec<syn::Pat> = params
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_r) => None,
            syn::FnArg::Typed(pat_type) => Some(pat_type.pat.as_ref().clone()),
        })
        .collect();

    let input_struct_name = input_struct_name_from_method(&method);

    let need_reply = maybe_return_types.is_some();

    let message_type = if need_reply {
        quote! { someip::MessageType::Request}
    } else {
        quote! {someip::MessageType::RequestNoReturn}
    };

    let timeout_ms: u64 = 1000;

    let call_and_reply_tokens = if need_reply {
        let (success_type, failure_type) = maybe_return_types.unwrap();
        quote! {
            let res = someip::client::Client::call(self.client.clone(),packet,std::time::Duration::from_millis(#timeout_ms)).await;
            //let res = client.call(packet,std::time::Duration::from_millis(#timeout_ms)).await;

            match res {
                Ok(someip::client::ReplyData::Completed(pkt)) => {
                    match pkt.header().message_type {
                        someip::someip_codec::MessageType::Response => {
                            // successful reply, deserialize the payload
                            let params_raw = pkt.payload().as_ref();
                            let maybe_response : Result<#success_type,_> = deserialize(params_raw);
                            if let Ok(response) = maybe_response {
                                Ok(response)
                            } else {
                                Err(MethodError::InvalidResponsePayload)
                            }
                        }
                        someip::someip_codec::MessageType::Request => {
                            log::error!("Proxy received Request packet. Ignored");
                            Err(MethodError::ConnectionError)
                        }
                        someip::someip_codec::MessageType::RequestNoReturn => {
                            log::error!("Proxy received RequestNoReturn packet. Ignored");
                            Err(MethodError::ConnectionError)
                        }
                        someip::someip_codec::MessageType::Notification => {
                            log::error!("Proxy received Notification packet as response. Ignored");
                            Err(MethodError::ConnectionError)
                        }
                        someip::someip_codec::MessageType::Error => {
                            // We need to deserialize the error type
                            let params_raw = pkt.payload().as_ref();
                            let maybe_response : Result<#failure_type,_> = deserialize(params_raw);
                            if let Ok(response) = maybe_response {
                                Err(MethodError::Error(response))
                            } else {
                                Err(MethodError::InvalidErrorPayload)
                            }
                        }
                    }
                }
                Ok(someip::client::ReplyData::Cancelled) => {
                    Err(MethodError::ConnectionError)
                }
                Ok(someip::client::ReplyData::Pending) => {
                    panic!("This should not happen");
                }
                Err(e) => {
                    Err(MethodError::ConnectionError)
                }
            } // end match
        }
    } else {
        quote! {
            //let client = self.client.read().unwrap();
            if let Err(_e) = someip::client::Client::call_noreply(self.client.clone(),packet).await {
                log::error!("call_noreply failed");
                Err(())
            } else {
                Ok(())
            }
        }
    };

    /// The method
    let tokens = quote! {
        pub async fn #ident ( #params ) #return_tokens {
            let input_params = #input_struct_name {
                #(#struct_members),*
            };

            let mut header = SomeIpHeader::default();
            header.set_method_id(#id);
            header.message_type = #message_type ;

            let input_raw = serialize(&input_params).unwrap();
            let payload = Bytes::from(input_raw);
            let packet = SomeIpPacket::new(header, payload);
            #call_and_reply_tokens
        }
    };
    tokens
}

fn create_get_field_method(field: &Field) -> syn::TraitItemMethod {
    let get_fn_name = format_ident!("get_{}", field.name);
    let field_type = &field.ty;
    let method_tokens = quote! {
        /// Get the value of the field. This method must be implemented
        /// by the server. The storage of the field may be implementation
        /// depentant. The method is called by the underlying binding when
        /// a client requests a property.
        fn #get_fn_name(&self) -> Result<&#field_type, someip::FieldError>;
    };

    //let parse_buffer: ParseBuffer = method_tokens.into();
    let method: syn::TraitItemMethod = syn::parse2(method_tokens).unwrap(); //  Signature::parse(&method_tokens.into());
    method
}

fn create_set_field_method(field: &Field) -> syn::TraitItemMethod {
    let set_fn_name = format_ident!("set_{}", field.name);
    let field_type = &field.ty;
    let field_name = &field.name;
    let method_tokens = quote! {
        fn #set_fn_name(&self,#field_name : #field_type ) -> Result<(), someip::FieldError>;
    };
    //let parse_buffer: ParseBuffer = method_tokens.into();
    let method: syn::TraitItemMethod = syn::parse2(method_tokens).unwrap(); //  Signature::parse(&method_tokens.into());
    method
}

fn create_send_event_method(field: &Field) -> syn::TraitItemMethod {
    let send_event_fn_name = format_ident!("send_event_{}", field.name);
    let field_type = &field.ty;
    let field_name = &field.name;
    let method_tokens = quote! {
        fn #send_event_fn_name(&self,#field_name : #field_type ) -> Result<(), io::Error> {
            todo!("Implement event sender");
            Ok(())
        }
    };
    //println!("Send Event Method:{}", &method_tokens.to_string());
    //let parse_buffer: ParseBuffer = method_tokens.into();
    let method: syn::TraitItemMethod = syn::parse2(method_tokens).unwrap(); //  Signature::parse(&method_tokens.into());
    method
}

fn get_methods(service: &Service) -> Vec<syn::TraitItemMethod> {
    let mut methods = Vec::new();
    for entry in &service.entries {
        match entry.ident.to_string().as_str() {
            "fields" => {
                for field in &entry.fields {
                    //println!("Field : {:?}:{}", field.id.id, field.name.to_string());
                    let set_method = create_set_field_method(field);
                    methods.push(set_method);

                    let get_method = create_get_field_method(field);
                    methods.push(get_method);
                }
            }
            "events" => {
                for field in &entry.fields {
                    //println!("Field : {:?}:{}", field.id.id, field.name.to_string());
                    let send_event_method = create_send_event_method(field);
                    methods.push(send_event_method);
                }
            }

            _ => {
                println!("{} unsupported", entry.ident.to_string());
            }
        }
    }
    methods
}

fn find_method_by_name<'a>(
    method_name: &str,
    item_trait: &'a syn::ItemTrait,
) -> Option<&'a syn::TraitItemMethod> {
    for item in &item_trait.items {
        if let syn::TraitItem::Method(m) = item {
            if m.sig.ident.to_string().as_str() == method_name {
                return Some(m);
            }
        }
    }
    None
}

fn find_method_by_ident<'a>(
    method_name: &Ident,
    item_trait: &'a syn::ItemTrait,
) -> Option<&'a syn::TraitItemMethod> {
    for item in &item_trait.items {
        if let syn::TraitItem::Method(m) = item {
            if &m.sig.ident == method_name {
                return Some(m);
            }
        }
    }
    None
}

fn create_deser_tokens(method_name: &str, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let method = find_method_by_name(method_name, item_trait).expect("Expecting method");
    let input_struct_name = input_struct_name_from_method(method);
    let method_name = &method.sig.ident;

    let params: Vec<syn::Pat> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_r) => None,
            syn::FnArg::Typed(pat_type) => Some(pat_type.pat.as_ref().clone()),
        })
        .collect();

    let call_method = quote! {
        let res_in_param : Result<#input_struct_name,_> = deserialize(params_raw);
        if res_in_param.is_err() {
            log::error!("Deserialization error for service:{} method:{}", pkt.header().service_id(), pkt.header().event_or_method_id());
            return Some(SomeIpPacket::error_packet_from(pkt, someip_codec::ReturnCode::NotOk, Bytes::new()));
        }
        let in_param = res_in_param.unwrap();
        let res = this.#method_name(#(in_param.#params),*);
    };

    call_method
}

fn create_field_getter_fn(field_name: &str, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let field_get_name = format!("get_{}", field_name);
    let method = find_method_by_name(&field_get_name, item_trait).expect("Expecting method");
    let method_name = &method.sig.ident;
    let params: Vec<syn::Pat> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_r) => None,
            syn::FnArg::Typed(pat_type) => Some(pat_type.pat.as_ref().clone()),
        })
        .collect();

    let call_method = quote! {
        this.#method_name()
    };

    call_method
}

fn create_field_setter_fn(field_name: &str, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let field_get_name = format!("set_{}", field_name);
    let method = find_method_by_name(&field_get_name, item_trait).expect("Expecting method");
    let method_name = &method.sig.ident;
    let params: Vec<syn::Type> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_r) => None,
            syn::FnArg::Typed(pat_type) => Some(pat_type.ty.as_ref().clone()),
        })
        .take(1)
        .collect();
    let field_type = &params[0];

    let call_method = quote! {
        let field: #field_type = deserialize(params_raw).unwrap();
        let res = this.#method_name (field);
    };

    call_method
}

fn get_method_ids(service: &Service) -> Vec<(u32, Ident)> {
    get_ids(service, "method_ids")
}

fn get_field_ids(service: &Service) -> Vec<(u32, Ident)> {
    get_ids(service, "events")
}

fn get_ids(service: &Service, id_type: &str) -> Vec<(u32, Ident)> {
    let mut ids: Vec<(u32, Ident)> = Vec::new();
    let fields: Vec<&ServiceEntry> = service
        .entries
        .iter()
        .filter_map(|e| {
            if e.ident.to_string().as_str() == id_type {
                Some(e)
            } else {
                None
            }
        })
        .collect();

    for field_entry in fields.iter() {
        for entry in field_entry.fields.iter() {
            ids.push((entry.id.id(), entry.name.clone()))
        }
    }

    ids
}

fn dispatch_method_call(id: u32, service: &Service, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let field = service.get_method_field(id).unwrap();

    todo!()
}

fn create_dispatch_handler(
    struct_name: &Ident,
    service: &Service,
    item_trait: &syn::ItemTrait,
) -> syn::Item {
    let method_id_name = get_method_ids(service);
    let method_ids: Vec<TokenStream2> = method_id_name
        .iter()
        .map(|(i, ident)| TokenStream2::from_str(&format!("{}", i)).unwrap())
        .collect();

    let mut deserialize_call = Vec::new();
    for e in &method_id_name {
        let deser_tokens = create_deser_tokens(e.1.to_string().as_ref(), item_trait);
        deserialize_call.push(deser_tokens);
    }

    let field_id_names = get_field_ids(service);
    let field_ids: Vec<TokenStream2> = field_id_names
        .iter()
        .map(|(i, ident)| TokenStream2::from_str(&format!("{}", i + 0x8000)).unwrap())
        .collect();

    let mut event_getters = Vec::new();
    for f in &field_id_names {
        let getter_tokens = create_field_getter_fn(f.1.to_string().as_ref(), item_trait);
        event_getters.push(getter_tokens);
    }

    let mut event_setters = Vec::new();
    for f in &field_id_names {
        let getter_tokens = create_field_setter_fn(f.1.to_string().as_ref(), item_trait);
        event_setters.push(getter_tokens);
    }

    let mut method_returns = Vec::new();
    for m in &method_id_name {
        let method = find_method_by_name(m.1.to_string().as_ref(), item_trait).unwrap();
        let method_reply_tokens = if method_has_return_type(&method) {
            quote! {
                match res {
                    Ok(r) => {
                        let reply_raw = serialize(&r).unwrap();
                        let reply_payload = Bytes::from(reply_raw);
                        Some(SomeIpPacket::reply_packet_from(pkt, someip_codec::ReturnCode::Ok, reply_payload))
                    }
                    Err(e) => {
                        let error_raw = serialize(&e).unwrap();
                        let error_payload = Bytes::from(error_raw);
                        Some(SomeIpPacket::error_packet_from(pkt, someip_codec::ReturnCode::NotOk, error_payload))
                    }
                }
            }
        } else {
            quote! {None}
        };
        method_returns.push(method_reply_tokens);
    }

    //let method_reply_tokens = if method_has_return_type()

    // We expect that there is a struct (or enum with the name  <trait>Server)
    let server_struct_name = format_ident!("{}ServerDispatcher", struct_name);
    let dispatch_tokens = quote! {
        pub mod dispatcher {
        use super::*;
            pub fn dispatch(this:&impl #struct_name, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
                match pkt.header().event_or_method_id() {
                    #(#method_ids => {
                        let params_raw = pkt.payload().as_ref();
                        #deserialize_call
                        #method_returns
                    })*
                    #(#field_ids => {
                        if pkt.payload().len() == 0 {
                            // get
                            let field = #event_getters .unwrap();
                            let reply_raw = serialize(&field).unwrap();
                            let reply_payload = Bytes::from(reply_raw);
                            Some(SomeIpPacket::reply_packet_from(
                                pkt,
                                someip_codec::ReturnCode::Ok,
                                reply_payload,
                            ))
                        } else {
                            //set
                            let params_raw = pkt.payload().as_ref();
                            #event_setters
                            match res {
                                Ok(r) => {
                                    let reply_raw = serialize(&r).unwrap();
                                    let reply_payload = Bytes::from(reply_raw);
                                    Some(SomeIpPacket::reply_packet_from(
                                        pkt,
                                        someip_codec::ReturnCode::Ok,
                                        reply_payload,
                                    ))
                                }
                                Err(e) => {
                                    // let reply_raw = serialize(&e).unwrap();
                                    // let reply_payload = Bytes::from(reply_raw);
                                    Some(SomeIpPacket::error_packet_from(
                                        pkt,
                                        someip_codec::ReturnCode::NotOk,
                                        Bytes::new(),
                                    ))
                                }
                            }
                        }
                    })*
                    _ => {
                        Some(SomeIpPacket::error_packet_from(pkt, someip_codec::ReturnCode::UnknownMethod, Bytes::new()))
                    }
                }
            }
        }// mod dispatcher
    };

    println!("dispatch function\n:{}", &dispatch_tokens.to_string());

    let method: syn::Item = syn::parse2(dispatch_tokens).unwrap();
    method
}

fn input_struct_name_from_method(m: &syn::TraitItemMethod) -> proc_macro2::Ident {
    format_ident!("Method{}Inputs", m.sig.ident)
}

// Add a super trait to this trait item.
fn add_super_trait(item_trait: &mut syn::ItemTrait) {
    let tokens = quote! {
        someip::server::ServerRequestHandler
    };
    let super_trait: syn::TypeParamBound = syn::parse2(tokens).unwrap();

    item_trait.supertraits.push(super_trait);
}

// create structures for marshalling and unmarshalling the input
// and output parameters
fn create_internal_marshalling_structs(item_trait: &syn::ItemTrait) -> Vec<syn::ItemStruct> {
    let mut item_structs = Vec::new();
    for item in &item_trait.items {
        if let TraitItem::Method(m) = &item {
            let mut input_args = Vec::new();
            for input in &m.sig.inputs {
                match input {
                    FnArg::Typed(typed) => input_args.push(typed.clone()),
                    FnArg::Receiver(r) => {}
                }
            }

            let struct_name = input_struct_name_from_method(m);
            let struct_tokens = quote! {
                #[derive(Serialize, Deserialize)]
                struct #struct_name {
                    #(#input_args),*
                }
            };
            let struct_item: syn::ItemStruct = syn::parse2(struct_tokens).unwrap();
            item_structs.push(struct_item);

            /// Now the output parameters
            if let syn::ReturnType::Type(_, ty) = &m.sig.output {
                match ty.as_ref() {
                    syn::Type::Path(p) => {
                        println!("Found type path");
                    }
                    _ => {}
                }
                //println!("Found type path");
            }
        }
    }
    item_structs
}

#[derive(Parse)]
struct Service {
    #[call(Punctuated::<ServiceEntry, Token![,]>::parse_terminated)]
    entries: Punctuated<ServiceEntry, Token![,]>,
}

impl Service {
    fn get_method_field(&self, id: u32) -> Option<&Field> {
        let service_entries: Vec<&ServiceEntry> = self
            .entries
            .iter()
            .filter_map(|e| {
                if e.ident.to_string().as_str() == "method_ids"
                    || e.ident.to_string().as_str() == "fields"
                {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        for entry in &service_entries {
            for field in &entry.fields {
                if field.id.id() == id {
                    return Some(field);
                }
            }
        }
        None
    }

    fn get_method_fields(&self) -> Vec<&Field> {
        self.get_fields_by_type("method_ids")
    }
    fn get_field_fields(&self) -> Vec<&Field> {
        self.get_fields_by_type("fields")
    }
    fn get_event_fields(&self) -> Vec<&Field> {
        self.get_fields_by_type("events")
    }

    fn get_fields_by_type(&self, ty: &str) -> Vec<&Field> {
        let service_entries: Vec<&ServiceEntry> = self
            .entries
            .iter()
            .filter_map(|e| {
                if e.ident.to_string().as_str() == ty {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        let mut res = Vec::new();
        for entry in &service_entries {
            for field in &entry.fields {
                res.push(field);
            }
        }
        res
    }

    //fn validate(&self) -> Option<>
}

#[derive(Parse)]
struct Id {
    id: syn::LitInt,
    arrow: Option<Token![=>]>,
    #[parse_if(arrow.is_some())]
    group_id: Option<syn::LitInt>,
}

impl Id {
    fn id(&self) -> u32 {
        let mut tokens = TokenStream2::new();
        self.id.to_tokens(&mut tokens);
        let id: u32 = tokens.to_string().parse().unwrap();
        id
    }

    fn group_id(&self) -> Option<u32> {
        let mut tokens = TokenStream2::new();
        self.group_id.as_ref().map(|e| {
            e.to_tokens(&mut tokens);
            let id: u32 = tokens.to_string().parse().unwrap();
            id
        })
    }
}

#[derive(Parse)]
struct Field {
    //id: Option<u32>,
    #[bracket]
    arg_paren: token::Bracket,
    #[inside(arg_paren)]
    id: Id,
    name: Ident,
    colon: Option<Token![:]>,
    #[parse_if(colon.is_some())]
    ty: Option<Type>,
}

impl Field {
    fn id(&self) -> u32 {
        self.id.id()
    }
}

struct Method {
    id: u32,
    name: Ident,
}

#[derive(Parse)]
struct ServiceEntry {
    ident: Ident,
    #[paren]
    arg_paren: token::Paren,
    #[inside(arg_paren)]
    #[call(Punctuated::<Field, Token![,]>::parse_terminated)]
    fields: Punctuated<Field, Token![,]>,
}
