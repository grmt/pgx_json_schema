use jsonschema::{output::BasicOutput, JSONSchema};
use pgx::prelude::*;
use pgx::JsonB;

#[pg_extern]
fn json_schema_is_valid(schema: JsonB, instance: JsonB) -> bool {
    jsonschema::is_valid(&schema.0, &instance.0)
}

#[pg_extern]
fn json_schema_get_validation_errors(
    schema: JsonB,
    instance: JsonB,
) -> TableIterator<
    'static,
    (
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
fn json_schema_get_errors_json(schema: JsonB, instance: JsonB) -> JsonB {
    let result = JSONSchema::compile(&schema.0)
        .unwrap_or_else(|err| panic!("Error compiling schema: {:#?}", err));

    let output: BasicOutput = result.apply(&instance.0).basic();

    JsonB(serde_json::to_value(output).unwrap())

}

#[pg_extern]
fn json_schema_get_errors(schema: JsonB, instance: JsonB)
 -> TableIterator<
'static, (
    name!(description, String),
    name!(instance_location, String),
    name!(keyword_location, String),
)
> {
let result = JSONSchema::compile(&schema.0)
        .unwrap_or_else(|err| panic!("Error compiling schema: {:#?}", err));

    let output: BasicOutput = result.apply(&instance.0).basic();


    let mut results: Vec<(String, String, String)> = Vec::new();

    match output {
        BasicOutput::Valid(annotations) => {
            for annotation in annotations {
                results.push((
                        annotation.value().to_string(),
                        annotation.instance_location().to_string(),
                        annotation.keyword_location().to_string(),
                    )
                );
                // println!(
                //     "Value: {} at path {}",
                //     annotation.value(),
                //     annotation.instance_location()
                // )
            }
        },
        BasicOutput::Invalid(errors) => {
            for error in errors {
                results.push((
                        error.error_description().to_string(),
                        error.instance_location().to_string(),
                        error.keyword_location().to_string(),
                    )
                );
                // println!(
                //     "Error: {} at path {}",
                //     error.error_description(),
                //     error.instance_location()
                // )
            }
        }
    }

    TableIterator::new(results.into_iter())


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
    fn test_json_schema_get_validation_errors() {
        let (_value, description) = Spi::get_two::<JsonB, String>(
            "SELECT * FROM json_schema_get_validation_errors('{\"maxLength\": 5}', '\"foobar\"'::jsonb)",
        );
        assert_eq!(description, Some("\"foobar\" is longer than 5 characters".to_string() )
        )
    }

    #[pg_test]
    fn test_json_schema_get_errors() {
        let (one, _two, _three) = Spi::get_three::<String, String, String>(
            "SELECT * FROM json_schema_get_errors('{\"maxLength\": 5}', '\"foobar\"'::jsonb)",
        );
        assert_eq!(
            one,
            Some("\"foobar\" is longer than 5 characters".to_string())
        )
    }

}
