use jsonschema::{output::BasicOutput, JSONSchema};
use pgx::prelude::*;
use pgx::JsonB;

#[pg_extern]
fn json_schema_is_valid(schema: JsonB, instance: JsonB) -> bool {
    jsonschema::is_valid(&schema.0, &instance.0)
}

#[pg_extern]
fn json_schema_get_errors(
    schema: JsonB,
    instance: JsonB
) -> TableIterator<
    'static, (
        name!(error_value, JsonB),
        name!(description, String),
        name!(details, String),
        name!(instance_path, String),
        name!(schema_path, String)
    )
> {
    let parsed_schema = JSONSchema::compile(&schema.0)
        .unwrap_or_else(|err| panic!("Error compiling schema: {:#?}", err));

    let result = parsed_schema
        .validate(&instance.0)
        .err()
        .into_iter()
        .flat_map(|iter| iter);

    let errors: Vec<(JsonB, String, String, String, String)> = result
        .map(|e| {
            let description = e.to_string();
            (
                JsonB(e.instance.into_owned()),
                description,
                format!("{:?}", (e.kind)).clone(),
                e.instance_path.to_string(),
                e.schema_path.to_string()
            )
        })
        .collect();

    TableIterator::new(errors.into_iter())
}

#[pg_extern]
fn json_schema_get_error(schema: JsonB, instance: JsonB) -> JsonB {
    let result = JSONSchema::compile(&schema.0)
        .unwrap_or_else(|err| panic!("Error compiling schema: {:#?}", err));

    let output: BasicOutput = result.apply(&instance.0).basic();

    JsonB(serde_json::to_value(output).unwrap())

}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgx::*;

    #[pg_test]
    fn test_json_schema_is_valid() {
        let valid = Spi::get_one::<bool>(
            "SELECT json_schema_is_valid('{\"maxLength\": 5}', '\"foobar\"'::jsonb)",
        );
        assert_eq!(valid, Some(false))
    }


    #[pg_test]
    fn test_json_schema_get_error() {
        let valid = Spi::get_one::<String>(
            "SELECT json_schema_is_valid('{\"maxLength\": 5}', '\"foobar\"'::jsonb)",
        );
        assert_eq!(valid, Some("{\"valid\": false, \"errors\": [{\"error\": \"\\\"foobaasdfr\\\" is longer than 5 characters\", \"keywordLocation\": \"/maxLength\", \"instanceLocation\": \"\"}]}".to_string() )
        )
    }

    #[pg_test]
    fn test_json_schema_get_errors() {
        let (_value, description) = Spi::get_two::<JsonB, String>(
            "SELECT * from json_schema_get_errors('{\"maxLength\": 5}', '\"foobar\"'::jsonb)",
        );
        assert_eq!(
            description,
            Some("\"foobar\" is longer than 5 characters".to_string())
        )
    }

}
