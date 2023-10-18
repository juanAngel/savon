//! WSDL inspection helpers.

use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
};
use xmltree::Element;

#[derive(Debug)]
pub enum WsdlError {
    Parse(xmltree::ParseError),
    ElementNotFound(&'static str),
    AttributeNotFound(&'static str),
    NotAnElement,
    Empty,
}

impl From<xmltree::ParseError> for WsdlError {
    fn from(error: xmltree::ParseError) -> Self {
        WsdlError::Parse(error)
    }
}

/// WSDL document.
#[derive(Debug)]
pub struct Wsdl {
    pub name: String,
    pub target_namespace: String,

    pub types: BTreeMap<QualifiedTypename, Type>,
    pub messages: BTreeMap<String, Message>,
    pub operations: BTreeMap<String, Operation>,
}

#[derive(Debug, Clone)]
pub enum SimpleType {
    Base64Binary,
    Boolean,
    String,
    Float,
    Int,
    DateTime,
    Complex(QualifiedTypename),
}

#[derive(Debug, Clone)]
pub enum Occurence {
    Unbounded,
    Num(u32),
}

#[derive(Debug, Clone, Default)]
pub struct TypeAttribute {
    pub nillable: bool,
    pub min_occurs: Option<Occurence>,
    pub max_occurs: Option<Occurence>,
}

#[derive(Debug, Clone)]
pub struct ComplexType {
    pub fields: BTreeMap<String, (TypeAttribute, SimpleType)>,
}

/// A fully qualified type name, consisting of a namespace and type name
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct QualifiedTypename(String, String);

impl QualifiedTypename {
    fn as_tuple(&self) -> (&str, &str) {
        (&self.0, &self.1)
    }

    pub fn name(&self) -> &str {
        &self.1
    }
}

impl FromStr for QualifiedTypename {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');

        let namespace = parts.next().ok_or("invalid qualified name")?.to_owned();
        let name = parts.next().ok_or("invalid qualified name")?.to_owned();
        Ok(Self(namespace, name))
    }
}

impl std::fmt::Display for QualifiedTypename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}:{}", self.0, self.1))
    }
}

#[derive(Debug, Clone)]
pub enum Type {
    Simple(SimpleType),
    Complex(ComplexType),
    Import(String),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub part_name: String,
    pub part_element: String,
}

#[derive(Debug)]
pub struct Operation {
    pub name: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub faults: Option<Vec<String>>,
}

//FIXME: splitting the namespace is the naive way, we should keep the namespace
// and check for collisions instead
fn split_namespace(s: &str) -> &str {
    match s.find(':') {
        None => s,
        Some(index) => &s[index + 1..],
    }
}

fn qualified_type(s: &str, namespaces: &xmltree::Namespace, default_ns: &str) -> QualifiedTypename {
    match s.find(':') {
        None => QualifiedTypename(default_ns.to_owned(), s.to_owned()),
        Some(index) => {
            let ns = namespaces.get(&s[..index]).unwrap();
            QualifiedTypename(ns.to_owned(), s[index + 1..].to_owned())
        }
    }
}

fn parse_type_ref(name: &QualifiedTypename) -> SimpleType {
    match name.as_tuple() {
        ("http://www.w3.org/2001/XMLSchema", "boolean") => SimpleType::Boolean,
        ("http://www.w3.org/2001/XMLSchema", "string") => SimpleType::String,
        ("http://www.w3.org/2001/XMLSchema", "int") => SimpleType::Int,
        ("http://www.w3.org/2001/XMLSchema", "float") => SimpleType::Float,
        ("http://www.w3.org/2001/XMLSchema", "dateTime") => SimpleType::DateTime,
        ("http://www.w3.org/2001/XMLSchema", "base64Binary") => SimpleType::Base64Binary,
        ("http://www.w3.org/2001/XMLSchema", t) => {
            warn!("unhandled simple type: {t}");

            SimpleType::Complex(QualifiedTypename(
                "http://www.w3.org/2001/XMLSchema".to_string(),
                t.to_string(),
            ))
        }
        (n, s) => SimpleType::Complex(QualifiedTypename(n.to_string(), s.to_string())),
    }
}

fn parse_element(
    field: &Element,
    target_namespace: &str,
) -> Result<(TypeAttribute, SimpleType), WsdlError> {
    let field_name = field
        .attributes
        .get("name")
        .ok_or(WsdlError::AttributeNotFound("name"))?;
    let field_type = field
        .attributes
        .get("type")
        .ok_or(WsdlError::AttributeNotFound("type"))?;
    let nillable = match field.attributes.get("nillable").map(|s| s.as_str()) {
        Some("true") => true,
        Some("false") => false,
        _ => false,
    };

    let min_occurs = match field.attributes.get("minOccurs").map(|s| s.as_str()) {
        None => None,
        Some("unbounded") => Some(Occurence::Unbounded),
        Some(n) => Some(Occurence::Num(
            n.parse().expect("occurence should be a number"),
        )),
    };
    let max_occurs = match field.attributes.get("maxOccurs").map(|s| s.as_str()) {
        None => None,
        Some("unbounded") => Some(Occurence::Unbounded),
        Some(n) => Some(Occurence::Num(
            n.parse().expect("occurence should be a number"),
        )),
    };

    // Filter for redundant combinations.
    let (nillable, min_occurs, max_occurs) = match (nillable, min_occurs, max_occurs) {
        // Commonly seen, no reason for min=1 and max=1 to be explicitly specified.
        (false, Some(Occurence::Num(1)), Some(Occurence::Num(1))) => (false, None, None),
        // min=0 and max=1 just means this value is nillable.
        (false, Some(Occurence::Num(0)), Some(Occurence::Num(1))) => (true, None, None),
        // No need to be nillable _and_ a Vec
        (true, Some(Occurence::Num(0 | 1)), Some(Occurence::Unbounded)) => {
            (false, Some(Occurence::Num(0)), Some(Occurence::Unbounded))
        }
        // Pass on through the attributes.
        (nillable, min_occurs, max_occurs) => (nillable, min_occurs, max_occurs),
    };

    trace!("field {:?} -> {:?}", field_name, field_type);
    let type_attributes = TypeAttribute {
        nillable,
        min_occurs,
        max_occurs,
    };

    let simple_type = parse_type_ref(&qualified_type(
        field_type.as_str(),
        field.namespaces.as_ref().unwrap(),
        target_namespace,
    ));

    Ok((type_attributes, simple_type))
}

/// Reference: https://learn.microsoft.com/en-us/previous-versions/dotnet/netframework-4.0/ms256050(v=vs.100)
fn parse_simple_type(el: &Element, target_namespace: &str) -> Result<Type, WsdlError> {
    // Simply type inner may only be one of:
    // - restriction
    // - list
    // - union

    // <s:simpleType name="guid">
    //   <s:restriction base="s:string">
    //     <s:pattern value="[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}" />
    //   </s:restriction>
    // </s:simpleType>

    let inner = el.children[0].as_element().ok_or(WsdlError::NotAnElement)?;
    let base = match inner.name.as_str() {
        "restriction" => {
            // Extends a base type with certain restrictions.
            for _child in inner.children.iter().filter_map(|c| c.as_element()) {
                // https://learn.microsoft.com/en-us/previous-versions/dotnet/netframework-4.0/ms256057(v=vs.100)
                // TODO: Parse these...
            }

            let base_type = inner
                .attributes
                .get("base")
                .ok_or(WsdlError::AttributeNotFound("base"))?;

            parse_type_ref(&qualified_type(
                base_type,
                el.namespaces.as_ref().unwrap(),
                target_namespace,
            ))
        }
        _n => panic!("unhandled simpleType inner: {_n}"),
    };

    Ok(Type::Simple(base))
}

/// Reference: https://learn.microsoft.com/en-us/previous-versions/dotnet/netframework-4.0/ms256067(v=vs.100)
fn parse_complex_type(el: &Element, target_namespace: &str) -> Result<Type, WsdlError> {
    let mut fields = BTreeMap::new();
    for child in el.children.iter() {
        let child = child.as_element().ok_or(WsdlError::NotAnElement)?;

        match child.name.as_str() {
            "sequence" => {
                for field in child.children.iter().filter_map(|c| c.as_element()) {
                    // FIXME: dup code
                    let field_name = field
                        .attributes
                        .get("name")
                        .ok_or(WsdlError::AttributeNotFound("name"))?;

                    let field = parse_element(field, target_namespace)?;
                    fields.insert(field_name.to_string(), field);
                }
            }
            n => {
                trace!("unhandled complexType inner: {n}");
                continue;
            }
        }
    }

    Ok(Type::Complex(ComplexType { fields }))
}

fn parse_schema(
    schema: &Element,
    target_namespace: &str,
) -> Result<(HashSet<String>, BTreeMap<QualifiedTypename, Type>), WsdlError> {
    let mut types = BTreeMap::new();
    let mut imports = HashSet::new();

    // Now parse individual types.
    let elems = schema.children.iter().filter_map(|c| c.as_element());
    for elem in elems {
        trace!("type: {:#?}", elem);
        let inner_type = match elem.name.as_str() {
            // sometimes we have <element name="TypeName"><complexType>...</complexType></element>,
            // sometimes we have <complexType name="TypeName">...</complexType>
            "element" => elem
                .children
                .get(0)
                .ok_or(WsdlError::Empty)?
                .as_element()
                .ok_or(WsdlError::NotAnElement)?,
            "complexType" => elem,
            "simpleType" => elem,
            // ```
            // <s:schema elementFormDefault="qualified" targetNamespace="http://www.microsoft.com/SoftwareDistribution">
            //   <s:import namespace="http://microsoft.com/wsdl/types/" />
            //   // ..... types that may refer to other namespace
            // </s:schema>
            // ```
            "import" => {
                let tns = elem
                    .attributes
                    .get("namespace")
                    .ok_or(WsdlError::AttributeNotFound("namespace"))?;

                imports.insert(tns.clone());
                continue;
            }
            n => {
                unimplemented!("unhandled type {n}");
            }
        };

        let name = elem
            .attributes
            .get("name")
            .ok_or(WsdlError::AttributeNotFound("name"))?;

        let new_type = match inner_type.name.as_str() {
            "complexType" => parse_complex_type(inner_type, target_namespace)?,
            "simpleType" => parse_simple_type(inner_type, target_namespace)?,
            n => unimplemented!("unhandled type {n}"),
        };

        types.insert(
            QualifiedTypename(target_namespace.to_string(), name.to_string()),
            new_type,
        );
    }

    Ok((imports, types))
}

pub fn parse_types(
    root_el: &Element,
    target_namespace: &str,
) -> Result<BTreeMap<QualifiedTypename, Type>, WsdlError> {
    let mut types = BTreeMap::new();

    let schemas = root_el.children.iter().filter_map(|c| c.as_element());
    for schema in schemas {
        let target_namespace = if let Some(ns) = schema.attributes.get("targetNamespace") {
            ns
        } else {
            target_namespace
        };

        // HACK: Ignoring imports for now and just flattening the namespaces.
        let (_imports, new_types) = parse_schema(schema, target_namespace)?;

        types.extend(new_types.into_iter());
    }

    Ok(types)
}

pub fn parse(bytes: &[u8]) -> Result<Wsdl, WsdlError> {
    let mut messages = BTreeMap::new();
    let mut operations = BTreeMap::new();
    let mut target_namespace = Vec::new();

    let elements = Element::parse(bytes)?;
    trace!("elements: {:#?}", elements);
    target_namespace.push(
        elements
            .attributes
            .get("targetNamespace")
            .ok_or(WsdlError::AttributeNotFound("targetNamespace"))?
            .to_string(),
    );

    let types_el = elements
        .get_child("types")
        .ok_or(WsdlError::ElementNotFound("types"))?;

    let default_ns = target_namespace.last().unwrap();
    let types = parse_types(types_el, default_ns)?;

    for message in elements
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|c| c.name == "message")
    {
        trace!("message: {:#?}", message);
        let name = message
            .attributes
            .get("name")
            .ok_or(WsdlError::AttributeNotFound("name"))?;

        let c = message
            .children
            .iter()
            .filter_map(|c| c.as_element())
            .next()
            .unwrap();
        //FIXME: namespace
        let part_name = c
            .attributes
            .get("name")
            .ok_or(WsdlError::AttributeNotFound("name"))?
            .to_string();
        let part_element = split_namespace(
            c.attributes
                .get("element")
                .ok_or(WsdlError::AttributeNotFound("element"))?,
        )
        .to_string();

        messages.insert(
            name.to_string(),
            Message {
                part_name,
                part_element,
            },
        );
    }

    let port_type_el = elements
        .get_child("portType")
        .ok_or(WsdlError::ElementNotFound("portType"))?;

    for operation in port_type_el.children.iter().filter_map(|c| c.as_element()) {
        let operation_name = operation
            .attributes
            .get("name")
            .ok_or(WsdlError::AttributeNotFound("name"))?;

        let mut input = None;
        let mut output = None;
        let mut faults = None;
        for child in operation
            .children
            .iter()
            .filter_map(|c| c.as_element())
            .filter(|c| c.attributes.get("message").is_some())
        {
            let message = split_namespace(
                child
                    .attributes
                    .get("message")
                    .ok_or(WsdlError::AttributeNotFound("message"))?,
            );

            // FIXME: not testing for unicity
            match child.name.as_str() {
                "input" => input = Some(message.to_string()),
                "output" => output = Some(message.to_string()),
                "fault" => {
                    if faults.is_none() {
                        faults = Some(Vec::new());
                    }
                    if let Some(v) = faults.as_mut() {
                        v.push(message.to_string());
                    }
                }
                _ => return Err(WsdlError::ElementNotFound("operation member")),
            }
        }

        operations.insert(
            operation_name.to_string(),
            Operation {
                name: operation_name.to_string(),
                input,
                output,
                faults,
            },
        );
    }

    //FIXME: ignoring bindings for now
    //FIXME: ignoring service for now
    let service_name = elements
        .get_child("service")
        .ok_or(WsdlError::ElementNotFound("service"))?
        .attributes
        .get("name")
        .ok_or(WsdlError::AttributeNotFound("name"))?;

    debug!("service name: {}", service_name);
    debug!("parsed types: {:#?}", types);
    debug!("parsed messages: {:#?}", messages);
    debug!("parsed operations: {:#?}", operations);

    Ok(Wsdl {
        name: service_name.to_string(),
        target_namespace: target_namespace.last().unwrap().clone(),
        types,
        messages,
        operations,
    })
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    const WIKIPEDIA_WSDL: &[u8] = include_bytes!("../../assets/wikipedia-example.wsdl");
    const EXAMPLE_WSDL: &[u8] = include_bytes!("../../assets/example.wsdl");

    #[test]
    fn parse_example() {
        let res = parse(EXAMPLE_WSDL);
        println!("res: {:?}", res);
        res.unwrap();
    }
}
