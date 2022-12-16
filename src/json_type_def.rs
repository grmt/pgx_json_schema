use jtd::Schema;
use pgx::prelude::*;
use pgx::JsonB;

#[pg_extern]
fn jtd_is_valid(schema: JsonB, instance: JsonB) -> bool {
    let parsed_schema =
        Schema::from_serde_schema(serde_json::from_value(schema.0).unwrap()).unwrap();
    let result = jtd::validate(&parsed_schema, &instance.0, Default::default()).unwrap();

    result.is_empty()
}

#[pg_extern]
fn jtd_get_errors(
    schema: JsonB,
    instance: JsonB,
) -> TableIterator<'static, (name!(instance_path, String), name!(schema_path, String))> {
    let parsed_schema =
        Schema::from_serde_schema(serde_json::from_value(schema.0).unwrap()).unwrap();
    let result = jtd::validate(&parsed_schema, &instance.0, Default::default()).unwrap();

    let new: Vec<(String, String)> = result
        .into_iter()
        .map(|e| {
            let (instance_path, schema_path) = e.into_owned_paths();
            (
                if instance_path.is_empty() {
                    String::new()
                } else {
                    "/".to_owned() + &instance_path.join("/")
                },
                if schema_path.is_empty() {
                    String::new()
                } else {
                    "/".to_owned() + &schema_path.join("/")
                },
            )
        })
        .collect();

    TableIterator::new(new.into_iter())
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_jtd_is_valid() {
        let valid = Spi::get_one::<bool>(
            r#"
            select jtd_is_valid('{
                "properties": {
                    "name": { "type": "string" },
                    "age": { "type": "uint32" },
                    "phones": {
                        "elements": {
                            "type": "string"
                        }
                    }
                }
            }'::jsonb, '{
                "age": "43",
                "phones": ["+44 1234567", 442345678]
            }'::jsonb)"#,
        );
        assert_eq!(valid, Some(false))
    }

    #[pg_test]
    fn test_jtd_get_errors() {
        let (instance_path, schema_path) = Spi::get_two::<String, String>(
            r#"
            select instance_path, schema_path from jtd_get_errors('{
                "properties": {
                    "name": { "type": "string" },
                    "age": { "type": "uint32" },
                    "phones": {
                        "elements": {
                            "type": "string"
                        }
                    }
                }
            }', '{
                "age": "43",
                "phones": ["+44 1234567", 442345678]
            }'::jsonb)"#,
        );

        assert_eq!(instance_path, Some("/age".to_string()));
        assert_eq!(schema_path, Some("/properties/age/type".to_string()));
    }
}
