use derive_syn_parse::Parse;
use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::str::FromStr;
use syn::parse::{Parse, ParseBuffer, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Expr, Signature, *};

fn create_get_field_method(field: &Field) -> syn::TraitItemMethod {
    let get_fn_name = format_ident!("get_{}", field.name);
    let field_type = &field.ty;
    let method_tokens = quote! {
        /// Get the value of the field. This method must be implemented
        /// by the server. The storage of the field may be implementation
        /// depentant. The method is called by the underlying binding when
        /// a client requests a property.
        fn #get_fn_name(&self) -> Result<&#field_type, io::Error>;
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
        fn #set_fn_name(&self,#field_name : #field_type ) -> Result<(), io::Error>;
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
        let in_param : #input_struct_name = deserialize(params_raw).unwrap();
        let res = self.#method_name(#(in_param.#params),*);
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
        self.#method_name()
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
        let res = self.#method_name (field);
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
) -> syn::ItemImpl {
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

    let dispatch_tokens = quote! {
        impl someip::server::ServerRequestHandler for #struct_name {
            fn handle(&self, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
                match pkt.header().event_or_method_id() {
                    #(#method_ids => {
                        let params_raw = pkt.payload().as_ref();
                        #deserialize_call
                        match res {
                            Ok(r) => {
                                let reply_raw = serialize(&r).unwrap();
                                let reply_payload = Bytes::from(reply_raw);
                                Some(SomeIpPacket::reply_packet_from(pkt, someip_codec::ReturnCode::Ok, reply_payload))
                            }
                            Err(e) => {
                                Some(SomeIpPacket::error_packet_from(pkt, someip_codec::ReturnCode::NotOk, Bytes::new()))
                            }
                        }
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
        }
    };

    println!("dispatch function\n:{}", &dispatch_tokens.to_string());

    let method: syn::ItemImpl = syn::parse2(dispatch_tokens).unwrap();
    method
}

fn input_struct_name_from_method(m: &syn::TraitItemMethod) -> proc_macro2::Ident {
    format_ident!("Method{}Inputs", m.sig.ident)
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

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());

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
    println!("GENERATED:{}", &token_stream.to_string());
    token_stream.into()
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
