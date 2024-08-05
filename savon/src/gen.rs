use crate::string;
use crate::wsdl::{parse, Occurence, QualifiedTypename, SimpleType, Type, Wsdl};
use case::CaseExt;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::ToTokens;
use std::{fs::File, io::Write};

pub trait ToElements {
    fn to_elements(&self) -> Vec<xmltree::Element>;
}

pub trait FromElement {
    fn from_element(element: &xmltree::Element) -> Result<Self, crate::Error>
    where
        Self: Sized;
}

impl<T: ToElements> ToElements for Option<T> {
    fn to_elements(&self) -> Vec<xmltree::Element> {
        match self {
            Some(e) => e.to_elements(),
            None => vec![],
        }
    }
}

/*impl<T: ToElements> for Vec<T> {
    fn to_elements(&self) -> Vec<xmltree::Element> {

        match self {
            Some(e) => e.to_elements(),
            None => vec![],
        }
    }
}*/

#[derive(Debug)]
pub enum GenError {
    Io(std::io::Error),
    OperationFieldRequired,
}

impl From<std::io::Error> for GenError {
    fn from(e: std::io::Error) -> Self {
        GenError::Io(e)
    }
}

fn gen_simple(ty: &SimpleType) -> TokenStream {
    match ty {
        SimpleType::Boolean => Ident::new("bool", Span::call_site()).to_token_stream(),
        SimpleType::String => Ident::new("String", Span::call_site()).to_token_stream(),
        SimpleType::Float => Ident::new("f64", Span::call_site()).to_token_stream(),
        SimpleType::Int => Ident::new("i64", Span::call_site()).to_token_stream(),
        SimpleType::Long => Ident::new("i64", Span::call_site()).to_token_stream(),
        SimpleType::DateTime => quote!{
            chrono::DateTime<chrono::Utc>
        },
        //SimpleType::DateTime => Ident::new("chrono::DateTime", Span::call_site()),
        SimpleType::Base64Binary => Ident::new("String", Span::call_site()).to_token_stream(), // TODO: Base64 type...
        SimpleType::Any => Ident::new("Any", Span::call_site()).to_token_stream(), // TODO: Any type...
        SimpleType::Complex(n) => Ident::new(&n.name().to_camel(), Span::call_site()).to_token_stream(),
    }
}
pub fn from_template<T>(element: &xmltree::Element) -> Result<Vec<T>, crate::Error>
        where T: crate::gen::FromElement + Clone
    {

    let schema = element.get_child("schema");
    let diffgram = element.get_child("diffgram").ok_or(crate::Error::Wsdl(crate::wsdl::WsdlError::NotAnElement))?;
    let doc = diffgram.get_child("DocumentElement").ok_or(crate::Error::Wsdl(crate::wsdl::WsdlError::NotAnElement))?;
    let doc_child = doc.children.iter();
    let mut result = vec![];
    for it in doc_child{
        let el = it.as_element().ok_or(crate::Error::Wsdl(crate::wsdl::WsdlError::NotAnElement))?;
        result.push(T::from_element(el)?);
    }

    Ok(result)
}

fn gen_type(name: &QualifiedTypename, t: &Type) -> TokenStream {
    let type_name = Ident::new(&name.name().to_camel(), Span::call_site());

    match t {
        Type::Complex(c) => {
            let fields = c
                .fields
                .iter()
                .map(|(field_name, (attributes, field_type))| {
                    let tgt = if let SimpleType::Complex(n) = field_type {
                        Some(n.clone())
                    } else {
                        None
                    };

                    let fname = Ident::new(&string::to_snake(field_name), Span::call_site());
                    let ft = gen_simple(field_type);
                    let ft = if attributes.template{
                        quote! { #ft::<T> }
                    }else{
                        quote! { #ft }
                    };

                    let ft = match (
                        attributes.min_occurs.as_ref(),
                        attributes.max_occurs.as_ref(),
                    ) {
                        (Some(_), Some(_)) => quote! { Vec<#ft> },
                        _ => quote! { #ft },
                    };
                    let ft = if attributes.nillable {
                        quote! { Option<#ft> }
                    } else {
                        ft
                    };

                    let docstr = if let Some(tgt) = tgt {
                        format!(" Qualified type: {tgt}")
                    } else {
                        String::new()
                    };

                    quote! {
                        #[doc = #docstr]
                        pub #fname: #ft,
                    }
                })
                .collect::<Vec<_>>();

            let fields_serialize_impl = c
                .fields
                .iter()
                .map(|(field_name, (attributes, field_type))| {
                    let fname = Ident::new(&string::to_snake(field_name), Span::call_site());
                    //FIXME: handle more complex types
                    /*let ft = match field_type {
                        SimpleType::Boolean => Ident::new("bool", Span::call_site()),
                        SimpleType::String => Ident::new("String", Span::call_site()),
                        SimpleType::Float => Ident::new("f64", Span::call_site()),
                        SimpleType::Int => Ident::new("i64", Span::call_site()),
                        SimpleType::Long => Ident::new("u64", Span::call_site()),
                        SimpleType::DateTime => Ident::new("String", Span::call_site()),
                        SimpleType::Complex(s) => Ident::new(&s, Span::call_site()),
                    };*/
                    let ftype = Literal::string(field_name);
                    let prefix = quote! { xmltree::Element::node(#ftype) };

                    match (
                        attributes.min_occurs.as_ref(),
                        attributes.max_occurs.as_ref(),
                    ) {
                        (Some(_), Some(_)) => {
                            match field_type{
                                SimpleType::Complex(_s) => if attributes.nillable {
                                    quote! {
                                        self.#fname.as_ref().map(|v| v.iter().map(|i| {
                                            #prefix.with_children(i.to_elements())
                                        }).collect::<Vec<_>>()).unwrap_or_else(Vec::new)
                                    }
                                } else {
                                    quote! {
                                        self.#fname.iter().map(|i| {
                                            #prefix.with_children(i.to_elements())
                                        }).collect::<Vec<_>>()
                                    }
                                },
                                SimpleType::DateTime => if attributes.nillable {
                                    quote! {
                                        self.#fname.as_ref().map(|v| v.iter().map(|i| {
                                            #prefix.with_text(i.to_rfc3339())
                                        }).collect::<Vec<_>>()).unwrap_or_else(Vec::new)
                                    }
                                } else {
                                    quote! {
                                        self.#fname.iter().map(|i| {
                                            #prefix.with_text(i.to_rfc3339())
                                        }).collect::<Vec<_>>()
                                    }
                                },
                                _ => if attributes.nillable {
                                    quote! {
                                        self.#fname.as_ref().map(|v| v.iter().map(|i| {
                                            #prefix.with_text(i.to_string())
                                        }).collect::<Vec<_>>()).unwrap_or_else(Vec::new)
                                    }
                                } else {
                                    quote! {
                                        self.#fname.iter().map(|i| {
                                            #prefix.with_text(i.to_string())
                                        }).collect::<Vec<_>>()
                                    }
                                },
                            }
                        }
                        _ => match field_type {
                            SimpleType::Complex(_s) => {
                                quote! { vec![#prefix.with_children(self.#fname.to_elements())]}
                            }
                            SimpleType::DateTime => if attributes.nillable{
                                quote! { 
                                    match self.#fname.as_ref(){
                                        Some(v) => vec![#prefix.with_text(v.to_rfc3339())],
                                        None => vec![]
                                    }
                                    
                                }
                            }else{
                                quote! { vec![#prefix.with_text(self.#fname.to_rfc3339())] }
                            },
                            _ => if attributes.nillable{
                                quote! { 
                                    match self.#fname.as_ref(){
                                        Some(v) => vec![#prefix.with_text(v.to_string())],
                                        None => vec![]
                                    }
                                    
                                }
                            }else{
                                quote! { vec![#prefix.with_text(self.#fname.to_string())] }
                            },
                        },
                    }
                })
                .collect::<Vec<_>>();

            let (template,template_namespace,where_clause) = if c.template{
                (quote!{<T>},quote!{::<T>},quote!{
                    where T: std::fmt::Debug + Default + Clone + PartialEq + savon::gen::FromElement + savon::gen::ToElements
                })
            }else{
                (quote!{},quote!{},quote!{})
            };

            let serialize_impl = if fields_serialize_impl.is_empty() {
                quote! {
                    impl #template savon::gen::ToElements for #type_name #template #where_clause{
                        fn to_elements(&self) -> Vec<xmltree::Element> {
                            vec![]
                        }
                    }
                }
            } else {
                quote! {
                    impl #template savon::gen::ToElements for #type_name #template #where_clause{
                        fn to_elements(&self) -> Vec<xmltree::Element> {
                            vec![#(#fields_serialize_impl),*].drain(..).flatten().collect()
                        }
                    }
                }
            };

            let fields_deserialize_impl = c
            .fields
            .iter()
            .map(|(field_name, (attributes, field_type))| {
                let fname = Ident::new(&string::to_snake(field_name), Span::call_site());
                let ftype = Literal::string(field_name);

                let prefix = quote!{ element.get_at_path(&[#ftype]) };

                let field = match field_type {
                    SimpleType::Base64Binary => {
                        // TODO: Properly parse this...
                        let ft = quote!{ #prefix.and_then(|e| e.get_text().map(|s| s.to_string())
                                              .ok_or(savon::rpser::xml::Error::Empty)
                                              ) };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else {
                            quote!{ #ft?,}
                        }
                    }
                    SimpleType::Boolean => {
                        let ft = quote!{ #prefix.and_then(|e| e.as_boolean()) };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else {
                            quote!{ #ft?,}
                        }
                    },
                    SimpleType::String => {
                        let ft = quote!{ #prefix.and_then(|e| e.get_text().map(|s| s.to_string())
                                             .ok_or(savon::rpser::xml::Error::Empty)
                                             ) };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else {
                            quote!{ #ft?,}
                        }
                    },
                    SimpleType::Float => {
                        let ft = quote!{ #prefix.map_err(savon::Error::from).and_then(|e| e.get_text()
                                             .ok_or(savon::rpser::xml::Error::Empty)
                                             .map_err(savon::Error::from)
                                             .and_then(|s| s.parse().map_err(savon::Error::from))) };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else if attributes.max_occurs == Some(Occurence::Unbounded){
                            quote!{ #ft?,}
                        } else {
                            quote!{ #ft?,}
                        }
                    },
                    SimpleType::Int => {
                        let ft = quote!{ #prefix.and_then(|e| e.as_long()) };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else {
                            quote!{ #ft?,}
                        }
                    },
                    SimpleType::Long => {
                        let ft = quote!{ #prefix.and_then(|e| e.as_long()) };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else if attributes.max_occurs == Some(Occurence::Unbounded){
                            quote!{ vec![#ft?],}
                        } else {
                            quote!{ #ft?,}
                        }
                    },
                    SimpleType::DateTime => {
                        let ft = quote!{
                            #prefix
                            .and_then(|e| e.get_text().map(|v|v.to_string())
                                             .ok_or(savon::rpser::xml::Error::Empty)
                            )
                            .map_err(savon::Error::from)
                            .and_then(|s|
                                      s.parse::<savon::internal::chrono::DateTime<savon::internal::chrono::offset::Utc>>().map_err(savon::Error::from))
                        };
                        if attributes.nillable {
                            quote!{ #ft.ok(),}
                        } else {
                            quote!{ #ft?,}
                        }
                    },
                    SimpleType::Complex(n) => {
                        let complex_type = Ident::new(&n.name().to_camel(), Span::call_site());

                        match (attributes.min_occurs.as_ref(), attributes.max_occurs.as_ref()) {
                            (Some(_), Some(_)) => {
                                let ft = quote! {
                                    {
                                        let mut v = vec![];
                                        for elem in element.children.iter()
                                            .filter_map(|c| c.as_element()) {
                                                v.push(#complex_type::from_element(elem)?);
                                            }
                                        v
                                    },
                                };

                                if attributes.min_occurs.as_ref().is_some_and(|v| match v {
                                        Occurence::Unbounded => true,
                                        Occurence::Num(v) => *v>1
                                }){
                                    quote!{ #fname: vec![#ft] }
                                }else if attributes.nillable {
                                    quote!{ #fname: Some(#ft) }
                                }else{
                                    quote!{ #fname: #ft }
                                }
                            },
                            _ => {
                                let ft = quote!{ #prefix.map_err(savon::Error::from).and_then(|e| #complex_type::from_element(&e).map_err(savon::Error::from)) };
                                if attributes.nillable {
                                    quote!{ #ft.ok(),}
                                } else {
                                    quote!{ #ft?,}
                                }
                            }
                        }
                    },
                    SimpleType::Any => quote!{}
                };
                quote!{#fname: #field}
            })
            .collect::<Vec<_>>();

            let deserialize_impl = if fields_deserialize_impl.is_empty() {
                quote! {
                    impl #template savon::gen::FromElement for #type_name #template  #where_clause{
                        fn from_element(_element: &xmltree::Element) -> Result<Self, savon::Error> {
                            Ok(#type_name #template_namespace {
                            })
                        }
                    }
                }
            } else {
                quote! {
                    impl #template savon::gen::FromElement for #type_name #template #where_clause{
                        fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                            Ok(#type_name #template_namespace {
                                #(#fields_deserialize_impl)*
                            })
                        }
                    }
                }
            };

            let docstr = format!(" Qualified type: {}", name);

            quote! {
                #[doc = #docstr]
                #[derive(Clone, Debug, Default,PartialEq)]
                pub struct #type_name #template #where_clause{
                    #(#fields)*
                }

                #serialize_impl

                #deserialize_impl
            }
        }
        Type::Simple(t) => {
            let ident = gen_simple(t);
            let field = quote! { pub #ident };

            // TODO: Serialize/deserialize impls
            let deserialize_impl = quote! {
                impl savon::gen::FromElement for #type_name {
                    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                        //TODO:
                        Ok(#type_name(element.get_text()
                            .ok_or(savon::rpser::xml::Error::Empty)
                            .map(|v|v.to_string())
                            .map_err(savon::Error::from)?
                        ))
                    }
                }
            };
            let serialize_impl = quote! {
                impl savon::gen::ToElements for #type_name {
                    fn to_elements(&self) -> Vec<xmltree::Element> {
                        //TODO:
                        vec![]
                    }
                }
            };

            let docstr = format!(" Qualified type: {}", name);

            quote! {
                #[doc = #docstr]
                #[derive(Clone, Debug, Default,PartialEq)]
                pub struct #type_name( #field );

                #serialize_impl

                #deserialize_impl
            }
        }
        Type::Template => {
            let docstr = format!(" Qualified type: {}", name);

            // TODO: Serialize/deserialize impls
            let deserialize_impl = quote! {
                impl<T> savon::gen::FromElement for #type_name<T> 
                    where T: Clone + std::fmt::Debug + Default + PartialEq + savon::gen::FromElement + savon::gen::ToElements{
                    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                        //TODO:
                        
                        Ok(#type_name(savon::gen::from_template(element)?))
                    }
                }
            };

            let serialize_impl = quote! {
                impl<T> savon::gen::ToElements for #type_name<T>
                    where T: Clone + std::fmt::Debug + Default + PartialEq  + savon::gen::FromElement + savon::gen::ToElements  {
                    fn to_elements(&self) -> Vec<xmltree::Element> {
                        //TODO:
                        vec![]
                    }
                }
            };

            quote! {
                #[doc = #docstr]
                #[derive(Clone, Debug, Default,PartialEq)]
                pub struct #type_name<T>( pub Vec<T> )
                    where T: Clone + std::fmt::Debug + Default + PartialEq + savon::gen::FromElement + savon::gen::ToElements;

                #serialize_impl

                #deserialize_impl
            }
        },
        Type::Enum(_) => panic!(),
        _ => panic!(),
    }
}

pub fn gen_write(path: &str, out: &str,file_name: &str) -> Result<(), crate::Error> {
    let out_path = format!("{}/{}.rs", out,file_name);
    let v = std::fs::read(path).map_err(|e|crate::Error::Io(e))?;
    let mut output = File::create(out_path).map_err(|e|crate::Error::Io(e))?;
    let wsdl = parse(&v[..]).map_err(|e|crate::Error::Wsdl(e))?;

    let generated = gen(&wsdl).map_err(|e|crate::Error::Gen(e.into()))?;
    let formatted = prettyplease::unparse(&syn::parse_quote!(#generated));

    output.write_all(formatted.as_bytes()).map_err(|e|crate::Error::Io(e))?;
    output.flush().map_err(|e|crate::Error::Io(e))?;

    Ok(())
}

pub fn gen(wsdl: &Wsdl) -> Result<TokenStream, GenError> {
    let target_namespace = Literal::string(&wsdl.target_namespace);

    let operations = wsdl.operations.iter().flat_map(|(name, operation)| {
        let op_name = Ident::new(&string::to_snake(name), Span::call_site());
        let input_name = match operation.input.as_ref(){
            Some(v) => Ident::new(&string::to_snake(v), Span::call_site()),
            None => return Err(GenError::OperationFieldRequired),
        };
        let input_type = match operation.input.as_ref(){
            Some(v) => Ident::new(&v.to_camel(), Span::call_site()),
            None => return Err(GenError::OperationFieldRequired),
        };

        let op_str = Literal::string(name);


        
        let (template,_template_in,template_in_namespace,_template_out,template_out_namespace,where_clause) = if operation.output_template && operation.input_template{
            (quote!{<I,O>},quote!{<I>},quote!{::<I>},quote!{<O>},quote!{::<O>},quote!{
                where I: std::fmt::Debug + Default + Clone + PartialEq + savon::gen::FromElement + savon::gen::ToElements,
                      O: std::fmt::Debug + Default + Clone + PartialEq + savon::gen::FromElement + savon::gen::ToElements
            })
        }else if operation.input_template{
            (quote!{<I>},quote!{<I>},quote!{::<I>},quote!{},quote!{},quote!{
                where I: std::fmt::Debug + Default + Clone + PartialEq + savon::gen::FromElement + savon::gen::ToElements
            })
        }else if operation.output_template{
            (quote!{<O>},quote!{},quote!{},quote!{<O>},quote!{::<O>},quote!{
                where O: std::fmt::Debug + Default + Clone + PartialEq + savon::gen::FromElement + savon::gen::ToElements
            })
        }else {
            (quote!{},quote!{},quote!{},quote!{},quote!{},quote!{})
        };

        Ok(match (operation.output.as_ref(), operation.faults.as_ref()) {
            (None, None) => {
                quote! {
                    pub async fn #op_name(&self, #input_name: #input_type) -> Result<(), savon::Error> {
                        savon::http::one_way(&self.client, &self.base_url, #target_namespace, #op_str, &#input_name).await
                    }
                }
            },
            (None, Some(_)) => quote!{},
            (Some(out), None) => {
                let out_name = Ident::new(out, Span::call_site());

                quote! {
                    pub async fn #op_name #template(&mut self, #input_name: #input_type #template_in_namespace) -> Result<Result<#out_name #template_out_namespace, ()>, savon::Error> #where_clause {
                        savon::http::request_response(&self.client, &self.base_url, &mut self.cookie,#target_namespace, #op_str, &#input_name).await
                    }
                }
            },
            (Some(out), Some(_)) => {
                let out_name = Ident::new(out, Span::call_site());
                let err_name = Ident::new(&format!("{}Error", name.to_camel()), Span::call_site());
                let input_name = match operation.input.as_ref() {
                    Some(v) => Ident::new(&string::to_snake(&format!("_{}", v)), Span::call_site()),
                    None => return Err(GenError::OperationFieldRequired),
                };

                quote! {
                    pub async fn #op_name(&self, #input_name: #input_type) -> Result<Result<#out_name, #err_name>, savon::Error> {
                        unimplemented!()
                        /*let req = hyper::http::request::Builder::new()
                            .method("POST")
                            .header("Content-Type", "text/xml-SOAP")
                            .header("MessageType", "Call")
                            .body(#input_name.as_xml())?;

                        let response: hyper::http::Response<String> = self.client.request(req).await?;
                        let body = response.body().await?;
                        if let Ok(out) = #out_name::from_xml(body) {
                            Ok(Ok(out))
                        } else {
                            Ok(#err_name::from_xml(body)?)
                        }
                        */
                    }
                }
            },
        })
    }).collect::<Vec<_>>();

    let types = wsdl
        .types
        .iter()
        .map(|(name, t)| gen_type(name, t))
        .collect::<Vec<_>>();

    let messages = wsdl
        .messages
        .iter()
        .map(|(message_name, message)| {
            let mname = Ident::new(message_name, Span::call_site());
            let iname = Ident::new(&message.part_element, Span::call_site());
            
            let (template,_template_namespace,where_clause) = if message.template{
                (quote!{<T>},quote!{::<T>},quote!{
                    where T: std::fmt::Debug + Default + Clone + PartialEq + savon::gen::FromElement + savon::gen::ToElements
                })
            }else{
                (quote!{},quote!{},quote!{})
            };

            quote! {
                #[derive(Clone, Debug, Default,PartialEq)]
                pub struct #mname #template(pub #iname #template) #where_clause;

                impl #template savon::gen::ToElements for #mname #template #where_clause{
                    fn to_elements(&self) -> Vec<xmltree::Element> {
                        self.0.to_elements()
                    }
                }

                impl #template savon::gen::FromElement for #mname #template #where_clause {
                    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                        #iname::from_element(element).map(#mname)
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let service_name = Ident::new(&wsdl.name, Span::call_site());

    let toks = quote! {
        use savon::internal::xmltree;
        #[allow(unused_imports)]
        use savon::rpser::xml::*;
        use savon::wsdl::WsdlError;

        #(#types)*

        pub struct #service_name {
            pub base_url: String,
            pub cookie: Option<String>,
            pub client: savon::internal::reqwest::Client,
        }

        #(#messages)*

        #[allow(dead_code)]
        impl #service_name {
            pub fn new(base_url: String) -> Self {
                Self::with_client(base_url, savon::internal::reqwest::Client::new())
            }

            pub fn with_client(base_url: String, client: savon::internal::reqwest::Client) -> Self {
                #service_name {
                    base_url,
                    cookie: None,
                    client,
                }
            }

            #(#operations)*
        }
    };

    let operation_faults = wsdl
        .operations
        .iter()
        .filter(|(_, op)| op.faults.is_some())
        .flat_map(|(name, operation)| {
            let op_error = Ident::new(&format!("{}Error", name), Span::call_site());

            let faults = match operation
                .faults.as_ref(){
                    Some(v) => v,
                    None => return Err(GenError::OperationFieldRequired)
                }
                .iter()
                .map(|fault| {
                    let fault_name = Ident::new(fault, Span::call_site());

                    quote! {
                          #fault_name(#fault_name),
                    }
                })
                .collect::<Vec<_>>();

            Ok(quote! {
                #[derive(Clone, Debug,PartialEq)]
                pub enum #op_error {
                    #(#faults)*
                }
            })
        })
        .collect::<Vec<_>>();

    let mut stream: TokenStream = toks;
    stream.extend(operation_faults);

    Ok(stream)
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    const WIKIPEDIA_WSDL: &[u8] = include_bytes!("../../assets/wikipedia-example.wsdl");
    const EXAMPLE_WSDL: &[u8] = include_bytes!("../../assets/example.wsdl");

    #[test]
    fn example() {
        let wsdl = parse(EXAMPLE_WSDL).unwrap();
        println!("wsdl: {:?}", wsdl);

        let res = gen(&wsdl).unwrap();

        println!("generated:\n{}", res);
    }
}
