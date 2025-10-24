use kolibrie::execute_query::execute_query;
use kolibrie::sparql_database::SparqlDatabase;

#[test]
fn test_kolibrie_database() {
    println!("Starting test_kolibrie_database");

    let mut db = SparqlDatabase::new();

    // Parse Turtle data
    let turtle_data = r#"
    @prefix ex: <http://example.org/> .

    ex:Alice ex:knows ex:Bob .
    ex:Bob ex:knows ex:Charlie .
    "#;
    println!("Parsing turtle data");
    db.parse_turtle(turtle_data);
    println!("Parsed turtle data");

    // Try matching the exact predicate from our data
    let sparql_query = r#"
        SELECT ?s ?o WHERE {
        ?s <http://example.org/knows> ?o .
        }"#;

    println!("Executing query");
    let results = execute_query(sparql_query, &mut db);
    println!("Query executed, results: {:?}", results);
    println!("Number of results: {}", results.len());

    for row in results {
        println!(
            "Subject: {}, Predicate: {}, Object: {}",
            row[0], row[1], row[2]
        );
    }
    println!("Test finished");
}
