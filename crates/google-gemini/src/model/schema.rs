use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberFormat {
    Float,
    Double,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegerFormat {
    Int32,
    Int64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StringFormat {
    Enum,
}

#[derive(Deserialize, Serialize)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nullable: Option<bool>,
    #[serde(flatten)]
    data: SchemaData,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SchemaData {
    Array {
        #[serde(skip_serializing_if = "Option::is_none")]
        items: Option<Box<Schema>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_items: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_items: Option<i64>,
    },
    Boolean,
    Integer {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<IntegerFormat>,
    },
    Number {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<NumberFormat>,
    },
    Object {
        #[serde(skip_serializing_if = "Option::is_none")]
        properties: Option<HashMap<String, Schema>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<Vec<String>>,
    },
    String {
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<StringFormat>,

        #[serde(rename = "enum")]
        variants: Option<Vec<String>>,
    },
    TypeUnspecified,
}

impl SchemaData {
    const fn new_number(format: NumberFormat) -> Self {
        Self::Number {
            format: Some(format),
        }
    }

    pub const fn new_f32() -> Self {
        Self::new_number(NumberFormat::Float)
    }

    pub const fn new_f64() -> Self {
        Self::new_number(NumberFormat::Double)
    }

    const fn new_integer(format: IntegerFormat) -> Self {
        Self::Integer {
            format: Some(format),
        }
    }

    pub const fn new_i32() -> Self {
        Self::new_integer(IntegerFormat::Int32)
    }

    pub const fn new_i64() -> Self {
        Self::new_integer(IntegerFormat::Int64)
    }

    pub const fn new_string() -> Self {
        Self::String {
            format: None,
            variants: None,
        }
    }

    pub fn new_enum(variants: &[&str]) -> Self {
        Self::String {
            format: Some(StringFormat::Enum),
            variants: Some(
                variants
                    .into_iter()
                    .map(|variant| variant.to_string())
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok() {
        let string = serde_json::to_string_pretty(&Schema {
            description: None,
            nullable: None,
            data: SchemaData::Object {
                properties: Some({
                    let mut properties = HashMap::new();

                    properties.insert(
                        "is_true".into(),
                        Schema {
                            description: None,
                            nullable: None,
                            data: SchemaData::Boolean,
                        },
                    );
                    properties.insert(
                        "is_false".into(),
                        Schema {
                            description: None,
                            nullable: None,
                            data: SchemaData::String {
                                format: Some(StringFormat::Enum),
                                variants: Some(vec!["maybe".into(), "idk".into()]),
                            },
                        },
                    );

                    properties
                }),
                required: Some(vec!["is_true".into()]),
            },
        })
        .unwrap();
    }
}
