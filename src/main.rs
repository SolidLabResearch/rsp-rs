use kolibrie::execute_query::execute_query;
use kolibrie::sparql_database::SparqlDatabase;

fn main() {
    let mut db = SparqlDatabase::new();

    // Parse Turtle data
    let turtle_data = r#"
    @prefix ex: <http://example.org/> .

    ex:Alice ex:age "30" .
    ex:Bob ex:age "25" .
    ex:Charlie ex:age "35" .
    "#;
    db.parse_turtle(turtle_data);

    // Execute a SPARQL SELECT query with FILTER and GROUP BY
    let sparql_query = r#"
    PREFIX ex: <http://example.org/>
    SELECT AVG(?age) AS ?averageAge
    WHERE {
        ?s ex:age ?age .
        FILTER (?age > "20")
    }
    GROUP BY ?averageAge
    "#;

    let results = execute_query(sparql_query, &mut db);

    for row in results {
        println!("Average Age: {}", row[0]);
    }
}
