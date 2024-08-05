use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, GenericArgument, PathArguments};

fn parseType(type_ident:syn::Type) -> Result<TokenStream,Error>{
    let type_token = if let syn::Type::Path(p) = &type_ident{
        let mut segments = p.path.segments.iter();
        let it = segments.next().expect("type empty");
        match it.ident.to_string().as_str() {
            "String" => {
                quote!{.map(|v|v.to_string())}
            },
            "Option" => {
                match it.arguments{
                    PathArguments::AngleBracketed(v) => {
                        let mut args = v.args.iter();

                        if let Some(GenericArgument::Type(syn::Type::Path(v))) = args.next(){
                            let mut segments = v.path.segments.iter();
                            if let Some(it) = segments.next(){
                                match it.ident.to_string().as_str() {
                                    "String" => {
                                        quote!{.map(|v|v.to_string())}
                                    },
                                    _ => {
                                        quote!{.map(|v|#type_ident::parse(v))}
                                    }
                                }
                            }else{
                                panic!("option empty");
                                quote!{}
                            }
                            
                        }else{
                            panic!("option empty");
                            quote!{}
                        }
                    },
                    _ => panic!("option empty")
                }
            },
            _ => {
                quote!{.map(|v|#type_ident::parse(v))}
            }
        }
    }else{
        quote!{}
    };
    Ok(type_token)
}

#[proc_macro_derive(XmlFromElement,attributes(redis_timestamp))]
pub fn xml_from_element(input: TokenStream) -> TokenStream{
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

	let struc_ident = &input.ident;
    let mut fields_parse = vec![];
    let mut fields_name = vec![];

    match &input.data{
		syn::Data::Struct(data) => {
            let fields = &data.fields;
            if let syn::Fields::Named(f) = fields {
                let fields_iter = f.named.iter();
                for it in fields_iter{
                    let name = it.ident.as_ref().unwrap();
                    let name_xml = name.to_string().to_uppercase();
                    let type_ident = it.ty.clone();
                    let type_token = parseType(type_ident).expect("");
                    
                    fields_name.push(
                        quote!{
                            #name,
                        }
                    );
                    let fl = quote!{
                        let #name = element.get_child(#name_xml)
                            .ok_or(savon::Error::Wsdl(savon::wsdl::WsdlError::ElementNotFound(#name_xml)))?
                            .get_text()#type_token;
                    };
                    fields_parse.push(fl);
                }
            }
        },
        _ => todo!("{:?}",&input.data)
    }

	let code = quote!{
        impl savon::gen::FromElement for #struc_ident{
			fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error>{
                #(#fields_parse)*

				Ok(Self{
                    #(#fields_name)*
                })
			}
		}
	};
    //println!("cargo:warn=code {:?}",code.to_string());
    code.into()
}