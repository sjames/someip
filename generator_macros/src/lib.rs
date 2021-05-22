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
        fn #set_fn_name(&mut self,#field_name : #field_type ) -> Result<(), io::Error>;
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

fn create_method_handler_match_case(service_entry: &ServiceEntry) -> TokenStream {
    todo!()
}

fn get_method_ids(service: &Service) -> Vec<u32> {
    get_ids(service, "method_ids")
}

fn get_field_ids(service: &Service) -> Vec<u32> {
    get_ids(service, "event_ids")
}

fn get_ids(service: &Service, id_type: &str) -> Vec<u32> {
    let mut ids: Vec<u32> = Vec::new();
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
            ids.push(entry.id.id())
        }
    }

    ids
}

fn create_dispatch_handler(service: &Service, item_trait: &syn::ItemTrait) -> syn::TraitItemMethod {
    let method_ids = get_method_ids(service);
    let method_ids: Vec<TokenStream2> = method_ids
        .iter()
        .map(|i| TokenStream2::from_str(&format!("{}", i)).unwrap())
        .collect();

    let dispatch_tokens = quote! {
        fn handle(&mut self, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
            match pkt.header().event_or_method_id() {
            #(#method_ids => {

            })*
            }
        }
    };

    println!("dispatch function\n:{}", &dispatch_tokens.to_string());

    let method: syn::TraitItemMethod = syn::parse2(dispatch_tokens).unwrap();
    method
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

            let struct_name = format_ident!("Method{}Inputs", m.sig.ident);
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

    let dispatch_handler = create_dispatch_handler(&service, &service_trait);

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

    println!("New Methods:{}", &token_stream.to_string());
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
                if e.ident.to_string().as_str() == "method_ids" {
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
}

#[derive(Parse)]
struct Id {
    id: ExprLit,
    arrow: Option<Token![=>]>,
    #[parse_if(arrow.is_some())]
    group_id: Option<ExprLit>,
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
