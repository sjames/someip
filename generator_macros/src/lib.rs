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
    let service = parse_macro_input!(attr as Service);
    let mut service_trait = parse_macro_input!(item as syn::ItemTrait);

    let service = create_method_ids(service, &service_trait);

    for entry in &service.entries {
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

    let new_methods = get_injected_methods(&service);
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
        //use someip::*;
        use bincode::{deserialize, serialize};
        use bytes::Bytes;
        #dispatch_handler
    };
    dispatch_handler_with_use.to_tokens(&mut token_stream);

    let proxy_tokens = create_proxy(&service, &service_trait);
    proxy_tokens.to_tokens(&mut token_stream);

    //println!("GENERATED:{}", &token_stream.to_string());
    token_stream.into()
}

// Find the next available method ID.  Skip over any used IDs until
// we get a free ID.
fn allocate_next_valid_method_id(
    mut service: Service,
    start: u32,
    method_name: &Ident,
) -> (u32, Service) {
    if get_method_ids(&service)
        .iter()
        .find(|(id, name)| *id == start)
        .is_some()
    {
        // id exists, try next one
        return allocate_next_valid_method_id(service, start + 1, method_name);
    } else {
        // inject a new ID.
        //let field_ts = quote! {method_ids([#start]#method_name)};
        let entry: ServiceEntry =
            syn::parse_str(&format!("method_ids([{}]{})", start, method_name)).unwrap();

        println!("Allocated method id {} to {}", start, method_name);

        service.entries.push(entry);
        (start + 1, service)
    }
}

/// create method Ids for methods without ids in the Service.
fn create_method_ids(service: Service, item_trait: &ItemTrait) -> Service {
    let mut current_id = 1;
    let mut current_service = service;
    for item in &item_trait.items {
        if let TraitItem::Method(m) = item {
            if get_method_ids(&current_service)
                .iter()
                .find(|(id, name)| name == &m.sig.ident)
                .is_none()
            {
                let (next_id, service) =
                    allocate_next_valid_method_id(current_service, current_id, &m.sig.ident);
                current_id = next_id;
                current_service = service;
            }
        }
    }
    current_service
}

fn create_proxy(service: &Service, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let struct_name = format_ident!("{}Proxy", item_trait.ident);
    let fields = get_fields(service);

    let mut field_name = Vec::new();
    for field in &fields {
        let name = field.name.clone();
        field_name.push(name);
    }

    let mut field_type = Vec::new();
    for field in &fields {
        let name = field.ty.as_ref().unwrap().clone();
        field_type.push(name);
    }

    let mut field_id = Vec::new();
    for field in &fields {
        let name = field.id.id.clone();
        field_id.push(name);
    }

    let mut tokens_struct = quote! {
        #[derive(Clone)]
        pub struct #struct_name {
            #(pub #field_name: Field<#field_type>,)*
            _client : Client,
        }

        impl #struct_name {
            pub fn new(service_id: u16, client_id: u16, config: Configuration) -> Self {
                let client = Client::new(service_id, client_id, config);
                #struct_name {
                    #(#field_name :  Field::new(#field_type::default(), client.clone(), #field_id ) ),*,
                    _client : client,
                }
            }
            pub async fn run(self, to: std::net::SocketAddr) -> Result<(), std::io::Error> {
                let client = {
                    let client = self._client.clone();
                    client
                };
                client.run(to).await
            }

            pub async fn run_uds(self, to: std::os::unix::net::UnixStream ) -> Result<(), std::io::Error> {
                let client = {
                    let client = self._client.clone();
                    client
                };
                client.run_uds(to).await
            }
        }
    };

    let id_idents = get_method_ids(service);

    let methods: Vec<TokenStream2> = id_idents
        .iter()
        .map(|(i, ident)| get_client_method_by_ident(*i as u16, ident, item_trait))
        .collect();

    let tokens_impl = quote! {
        impl #struct_name {
            #(#methods)*
        }
    };

    tokens_struct.extend(tokens_impl);

    tokens_struct.into()
}

fn get_fields(service: &Service) -> Vec<Field> {
    let mut fields = Vec::new();
    for entry in &service.entries {
        match entry.ident.to_string().as_str() {
            "fields" => {
                for field in &entry.fields {
                    fields.push(field.clone());
                }
            }
            _ => {
                //println!("{} ignored", entry.ident.to_string());
            }
        }
    }
    fields
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

fn method_is_async(method: &syn::TraitItemMethod) -> bool {
    method.sig.asyncness.is_some()
}

fn trait_is_async(trait_item: &ItemTrait) -> bool {
    for item in &trait_item.items {
        if let TraitItem::Method(m) = item {
            if method_is_async(&m) {
                return true;
            }
        }
    }
    return false;
}

/// Generate the code for a single RPC method in the trait
fn get_client_method_by_ident(id: u16, ident: &Ident, item_trait: &syn::ItemTrait) -> TokenStream2 {
    let method = find_method_by_ident(ident, item_trait).expect("Expected to find method");
    let params = &method.sig.inputs;

    let maybe_return_types = match method.sig.output.clone() {
        ReturnType::Type(_a, t) => {
            if let syn::Type::Path(p) = *t.clone() {
                if let Some((success_type, failure_type)) = get_result_types(&p) {
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
        quote! { MessageType::Request}
    } else {
        quote! {MessageType::RequestNoReturn}
    };

    let timeout_ms: u64 = 1000;

    let call_and_reply_tokens = if need_reply {
        let (success_type, failure_type) = maybe_return_types.unwrap();
        quote! {
            let res = self._client.call(packet,properties.timeout).await;

            match res {
                Ok(ReplyData::Completed(pkt)) => {
                    match pkt.header().message_type {
                        MessageType::Response => {
                            // successful reply, deserialize the payload
                            let params_raw = pkt.payload().as_ref();
                            let maybe_response : Result<#success_type,_> = deserialize(params_raw);
                            if let Ok(response) = maybe_response {
                                Ok(response)
                            } else {
                                Err(MethodError::InvalidResponsePayload)
                            }
                        }
                        MessageType::Request => {
                            log::error!("Proxy received Request packet. Ignored");
                            println!("Client call error1");
                            Err(MethodError::ConnectionError)
                        }
                        MessageType::RequestNoReturn => {
                            log::error!("Proxy received RequestNoReturn packet. Ignored");
                            println!("Client call error2");
                            Err(MethodError::ConnectionError)
                        }
                        MessageType::Notification => {
                            log::error!("Proxy received Notification packet as response. Ignored");
                            println!("Client call error3");
                            Err(MethodError::ConnectionError)
                        }
                        MessageType::Error => {
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
                Ok(ReplyData::Cancelled) => {
                    Err(MethodError::ConnectionError)
                }
                Ok(ReplyData::Pending) => {
                    panic!("This should not happen")
                }
                Err(e) => {
                    Err(MethodError::ConnectionError)
                }
            } // end match
        }
    } else {
        quote! {
            //let client = self.client.read().unwrap();
            if let Err(_e) = self._client.call_noreply(packet).await {
                log::error!("call_noreply failed");
                Err(())
            } else {
                Ok(())
            }
        }
    };

    // The method
    let tokens = quote! {
        pub async fn #ident ( #params , properties:&CallProperties) #return_tokens {
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
        fn #get_fn_name(&self) -> Result<&#field_type, FieldError>;
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
        fn #set_fn_name(&self,#field_name : #field_type ) -> Result<(), FieldError>;
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
        fn #send_event_fn_name(&self,#field_name : #field_type ) -> Result<(), std::io::Error> {
            todo!("Implement event sender");
            Ok(())
        }
    };
    //let parse_buffer: ParseBuffer = method_tokens.into();
    let method: syn::TraitItemMethod = syn::parse2(method_tokens).unwrap(); //  Signature::parse(&method_tokens.into());
    method
}

fn get_injected_methods(service: &Service) -> Vec<syn::TraitItemMethod> {
    let mut methods = Vec::new();
    for entry in &service.entries {
        match entry.ident.to_string().as_str() {
            "fields" => {
                for field in &entry.fields {
                    let set_method = create_set_field_method(field);
                    methods.push(set_method);

                    let get_method = create_get_field_method(field);
                    methods.push(get_method);
                }
            }
            "events" => {
                for field in &entry.fields {
                    let send_event_method = create_send_event_method(field);
                    methods.push(send_event_method);
                }
            }

            _ => {
                // Nothing needs to be done for methods
                //println!("{} unsupported", entry.ident.to_string());
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

    let is_async = method_is_async(&method);

    let params: Vec<syn::Pat> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_r) => None,
            syn::FnArg::Typed(pat_type) => Some(pat_type.pat.as_ref().clone()),
        })
        .collect();

    let await_if_needed = if is_async {
        quote! {.await}
    } else {
        quote! {}
    };

    let call_method = quote! {
        let res_in_param : Result<#input_struct_name,_> = deserialize(params_raw);
        if res_in_param.is_err() {
            log::error!("Deserialization error for service:{} method:{}", pkt.header().service_id(), pkt.header().event_or_method_id());
            return Some(SomeIpPacket::error_packet_from(pkt, ReturnCode::NotOk, Bytes::new()));
        }
        let in_param = res_in_param.unwrap();
        let res = this.#method_name(#(in_param.#params),*)#await_if_needed;
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

    let async_fn_if_needed = if trait_is_async(item_trait) {
        quote! {async}
    } else {
        quote! {}
    };

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
                        Some(SomeIpPacket::reply_packet_from(pkt, ReturnCode::Ok, reply_payload))
                    }
                    Err(e) => {
                        let error_raw = serialize(&e).unwrap();
                        let error_payload = Bytes::from(error_raw);
                        Some(SomeIpPacket::error_packet_from(pkt, ReturnCode::NotOk, error_payload))
                    }
                }
            }
        } else {
            quote! {None}
        };
        method_returns.push(method_reply_tokens);
    }

    let module_name = format_ident!("{}_dispatcher", struct_name);
    //let method_reply_tokens = if method_has_return_type()

    // We expect that there is a struct (or enum with the name  <trait>Server)
    //let server_struct_name = format_ident!("{}ServerDispatcher", struct_name);
    let dispatch_tokens = quote! {
        #[allow(non_snake_case)]
        pub mod #module_name {
        use super::*;
            pub async fn dispatch(this: std::sync::Arc<#struct_name>, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
            //pub async fn dispatch(this:&mut impl #struct_name, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
                //let mut this = this.lock().unwrap();
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
                                ReturnCode::Ok,
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
                                        ReturnCode::Ok,
                                        reply_payload,
                                    ))
                                }
                                Err(e) => {
                                    // let reply_raw = serialize(&e).unwrap();
                                    // let reply_payload = Bytes::from(reply_raw);
                                    Some(SomeIpPacket::error_packet_from(
                                        pkt,
                                        ReturnCode::NotOk,
                                        Bytes::new(),
                                    ))
                                }
                            }
                        }
                    })*
                    _ => {
                        Some(SomeIpPacket::error_packet_from(pkt, ReturnCode::UnknownMethod, Bytes::new()))
                    }
                }
            }
        }// mod dispatcher
    };

    //println!("dispatch function\n:{}", &dispatch_tokens.to_string());

    let method: syn::Item = syn::parse2(dispatch_tokens).unwrap();
    method
}

fn input_struct_name_from_method(m: &syn::TraitItemMethod) -> proc_macro2::Ident {
    format_ident!("Method{}Inputs", m.sig.ident)
}

// Add a super trait to this trait item.
fn add_super_trait(item_trait: &mut syn::ItemTrait) {
    let tokens = quote! {
        Send
    };
    let super_trait: syn::TypeParamBound = syn::parse2(tokens).unwrap();

    item_trait.supertraits.push(super_trait);

    let tokens = quote! {
        Sync
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
                #[allow(non_camel_case_types)]
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
                        //println!("Found type path");
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

#[derive(Parse, Clone)]
struct Id {
    id: Option<syn::LitInt>,
    arrow: Option<Token![;]>,
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

#[derive(Parse, Clone)]
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

/// A marker to apply on Service implementations
#[proc_macro_attribute]
pub fn service_impl(attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let mut service_trait = parse_macro_input!(item as syn::ItemStruct);

    let impl_name = &service_trait.ident;

    let service_impl = quote! {
    //        impl ServerRequestHandler for #impl_name {
    //            async fn handle(&mut self, message: SomeIpPacket) -> Option<SomeIpPacket> {
    //                None //dispatcher::dispatch(self, message).await
    //            }
     //       }
        };

    let mut token_stream = TokenStream2::new();
    service_trait.to_tokens(&mut token_stream);

    token_stream.extend(service_impl);

    token_stream.into()
}
