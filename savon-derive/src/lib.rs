use proc_macro::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Error, GenericArgument, PathArguments};

fn parse_type(field_name:&str,type_ident:syn::Type) -> Result<proc_macro2::TokenStream,Error>{
    let type_token = if let syn::Type::Path(p) = &type_ident{
        let segment = p.path.segments.last().expect("type empty");
        match segment.ident.to_string().as_str() {
            "String" => {
                quote!{
                    .map(|v|v.to_string())
                    .ok_or(savon::Error::Wsdl(savon::wsdl::WsdlError::Empty))?
                }
            },
            "Option" => {
                match &segment.arguments{
                    PathArguments::AngleBracketed(v) => {
                        let mut args = v.args.iter();
                        if let Some(GenericArgument::Type(v)) = args.next(){
                            let type_ident = quote!{#v};
                            println!("cargo:warn= type_ident {:?}",type_ident.to_string());

                            if let syn::Type::Path(p) = v{
                                let segment = p.path.segments.last().expect("type empty");

                                match segment.ident.to_string().as_str() {
                                    "String" => {
                                        quote!{.map(|v|v.to_string())}
                                    },
                                    _ => {
                                        quote!{
                                            .map(|v|v.parse::<#type_ident>()
                                                .map(|v|Some(v))
                                                .map_err(|e|savon::Error::ParseError(e.to_string())))
                                            .unwrap_or(Ok(None))?
                                        }
                                    }
                                }
                            }else{
                                return Err(Error::new(p.span(), "option empty"));
                            }
                        }else{
                            return Err(Error::new(p.span(), "option empty"));
                        }
                    },
                    _ => return Err(Error::new(segment.span(), "option empty"))
                }
            },
            _ => {
                quote!{
                    .map(|v|v.parse::<#type_ident>().map_err(|e|savon::Error::ParseError(e.to_string())))
                    .ok_or(savon::Error::Wsdl(savon::wsdl::WsdlError::Empty))??
                }
            }
        }
    }else{
        return Err(Error::new(type_ident.span(), "type unsuported"))
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
                    let type_token = parse_type(&name_xml,type_ident).expect("");
                    
                    fields_name.push(
                        quote!{
                            #name,
                        }
                    );
                    let fl = quote!{
                        let #name = element.get_child(#name_xml)
                            .and_then(|v|{
                                v.get_text()
                            })
                            #type_token;
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