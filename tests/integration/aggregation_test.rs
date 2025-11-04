use oxigraph::model::*;
use rsp_rs::{CSPARQLWindow, R2ROperator, ReportStrategy, Tick};
use std::sync::{Arc, Mutex};

#[test]
fn test_count_aggregation() {
    let window = Arc::new(Mutex::new(CSPARQLWindow::new(
        "http://example.org/tempWindow".to_string(),
        5000,
        1000,
        ReportStrategy::OnWindowClose,
        Tick::TimeDriven,
        0,
    )));

    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT (COUNT(?temperature) AS ?count)
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#
    .to_string();

    let r2r = R2ROperator::new(query);
    let result_count = Arc::new(Mutex::new(0));
    let result_count_clone = result_count.clone();

    {
        let mut win = window.lock().unwrap();
        win.subscribe(rsp_rs::StreamType::RStream, move |container| {
            match r2r.execute(&container) {
                Ok(results) => {
                    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                        for solution in solutions {
                            if let Ok(_sol) = solution {
                                *result_count_clone.lock().unwrap() += 1;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Add test data
    for i in 1..=3 {
        let timestamp = i * 1000;
        let quad = Quad::new(
            NamedNode::new("http://example.org/sensor1").unwrap(),
            NamedNode::new("http://example.org/temperature").unwrap(),
            Literal::new_typed_literal(
                &format!("{}", 20 + i),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
            ),
            GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
        );

        let mut win = window.lock().unwrap();
        win.add(quad, timestamp);
    }

    // Trigger window close
    {
        let mut win = window.lock().unwrap();
        win.add(
            Quad::new(
                NamedNode::new("http://example.org/dummy").unwrap(),
                NamedNode::new("http://example.org/dummy").unwrap(),
                Literal::new_simple_literal("dummy"),
                GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
            ),
            6000,
        );
    }

    assert!(
        *result_count.lock().unwrap() > 0,
        "COUNT query should return results"
    );
}

#[test]
fn test_avg_aggregation() {
    let window = Arc::new(Mutex::new(CSPARQLWindow::new(
        "http://example.org/tempWindow".to_string(),
        5000,
        1000,
        ReportStrategy::OnWindowClose,
        Tick::TimeDriven,
        0,
    )));

    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT (AVG(?temperature) AS ?avgTemp)
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#
    .to_string();

    let r2r = R2ROperator::new(query);
    let result_count = Arc::new(Mutex::new(0));
    let result_count_clone = result_count.clone();

    {
        let mut win = window.lock().unwrap();
        win.subscribe(rsp_rs::StreamType::RStream, move |container| {
            match r2r.execute(&container) {
                Ok(results) => {
                    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                        for solution in solutions {
                            if let Ok(_sol) = solution {
                                *result_count_clone.lock().unwrap() += 1;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Add test data
    for i in 1..=4 {
        let timestamp = 10000 + i * 1000;
        let quad = Quad::new(
            NamedNode::new("http://example.org/sensor1").unwrap(),
            NamedNode::new("http://example.org/temperature").unwrap(),
            Literal::new_typed_literal(
                &format!("{}", i * 10),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
            ),
            GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
        );

        let mut win = window.lock().unwrap();
        win.add(quad, timestamp);
    }

    // Trigger window close
    {
        let mut win = window.lock().unwrap();
        win.add(
            Quad::new(
                NamedNode::new("http://example.org/dummy").unwrap(),
                NamedNode::new("http://example.org/dummy").unwrap(),
                Literal::new_simple_literal("dummy"),
                GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
            ),
            16000,
        );
    }

    assert!(
        *result_count.lock().unwrap() > 0,
        "AVG query should return results"
    );
}

#[test]
fn test_min_max_aggregation() {
    let window = Arc::new(Mutex::new(CSPARQLWindow::new(
        "http://example.org/tempWindow".to_string(),
        5000,
        1000,
        ReportStrategy::OnWindowClose,
        Tick::TimeDriven,
        0,
    )));

    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT (MIN(?temperature) AS ?minTemp) (MAX(?temperature) AS ?maxTemp)
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#
    .to_string();

    let r2r = R2ROperator::new(query);
    let result_count = Arc::new(Mutex::new(0));
    let result_count_clone = result_count.clone();

    {
        let mut win = window.lock().unwrap();
        win.subscribe(rsp_rs::StreamType::RStream, move |container| {
            match r2r.execute(&container) {
                Ok(results) => {
                    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                        for solution in solutions {
                            if let Ok(_sol) = solution {
                                *result_count_clone.lock().unwrap() += 1;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Add test data
    let values = vec![15, 42, 8, 31, 23];
    for (i, val) in values.iter().enumerate() {
        let timestamp = 20000 + (i as i64 + 1) * 1000;
        let quad = Quad::new(
            NamedNode::new("http://example.org/sensor1").unwrap(),
            NamedNode::new("http://example.org/temperature").unwrap(),
            Literal::new_typed_literal(
                &format!("{}", val),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
            ),
            GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
        );

        let mut win = window.lock().unwrap();
        win.add(quad, timestamp);
    }

    // Trigger window close
    {
        let mut win = window.lock().unwrap();
        win.add(
            Quad::new(
                NamedNode::new("http://example.org/dummy").unwrap(),
                NamedNode::new("http://example.org/dummy").unwrap(),
                Literal::new_simple_literal("dummy"),
                GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
            ),
            26000,
        );
    }

    assert!(
        *result_count.lock().unwrap() > 0,
        "MIN/MAX query should return results"
    );
}

#[test]
fn test_sum_aggregation() {
    let window = Arc::new(Mutex::new(CSPARQLWindow::new(
        "http://example.org/tempWindow".to_string(),
        5000,
        1000,
        ReportStrategy::OnWindowClose,
        Tick::TimeDriven,
        0,
    )));

    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT (SUM(?temperature) AS ?totalTemp)
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#
    .to_string();

    let r2r = R2ROperator::new(query);
    let result_count = Arc::new(Mutex::new(0));
    let result_count_clone = result_count.clone();

    {
        let mut win = window.lock().unwrap();
        win.subscribe(rsp_rs::StreamType::RStream, move |container| {
            match r2r.execute(&container) {
                Ok(results) => {
                    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                        for solution in solutions {
                            if let Ok(_sol) = solution {
                                *result_count_clone.lock().unwrap() += 1;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Add test data
    for i in 1..=5 {
        let timestamp = 30000 + i * 1000;
        let quad = Quad::new(
            NamedNode::new("http://example.org/sensor1").unwrap(),
            NamedNode::new("http://example.org/temperature").unwrap(),
            Literal::new_typed_literal(
                &format!("{}", i * 5),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
            ),
            GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
        );

        let mut win = window.lock().unwrap();
        win.add(quad, timestamp);
    }

    // Trigger window close
    {
        let mut win = window.lock().unwrap();
        win.add(
            Quad::new(
                NamedNode::new("http://example.org/dummy").unwrap(),
                NamedNode::new("http://example.org/dummy").unwrap(),
                Literal::new_simple_literal("dummy"),
                GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
            ),
            36000,
        );
    }

    assert!(
        *result_count.lock().unwrap() > 0,
        "SUM query should return results"
    );
}

#[test]
fn test_group_by_aggregation() {
    let window = Arc::new(Mutex::new(CSPARQLWindow::new(
        "http://example.org/tempWindow".to_string(),
        5000,
        1000,
        ReportStrategy::OnWindowClose,
        Tick::TimeDriven,
        0,
    )));

    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?sensor (AVG(?temperature) AS ?avgTemp) (COUNT(?temperature) AS ?count)
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
        GROUP BY ?sensor
    "#
    .to_string();

    let r2r = R2ROperator::new(query);
    let result_count = Arc::new(Mutex::new(0));
    let result_count_clone = result_count.clone();

    {
        let mut win = window.lock().unwrap();
        win.subscribe(rsp_rs::StreamType::RStream, move |container| {
            match r2r.execute(&container) {
                Ok(results) => {
                    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                        for solution in solutions {
                            if let Ok(_sol) = solution {
                                *result_count_clone.lock().unwrap() += 1;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Add test data from multiple sensors
    let sensors = vec!["sensor1", "sensor2", "sensor1", "sensor2", "sensor1"];
    let temps = vec![20, 25, 22, 27, 24];

    for (i, (sensor, temp)) in sensors.iter().zip(temps.iter()).enumerate() {
        let timestamp = 40000 + (i as i64 + 1) * 1000;
        let quad = Quad::new(
            NamedNode::new(&format!("http://example.org/{}", sensor)).unwrap(),
            NamedNode::new("http://example.org/temperature").unwrap(),
            Literal::new_typed_literal(
                &format!("{}", temp),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
            ),
            GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
        );

        let mut win = window.lock().unwrap();
        win.add(quad, timestamp);
    }

    // Trigger window close
    {
        let mut win = window.lock().unwrap();
        win.add(
            Quad::new(
                NamedNode::new("http://example.org/dummy").unwrap(),
                NamedNode::new("http://example.org/dummy").unwrap(),
                Literal::new_simple_literal("dummy"),
                GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
            ),
            46000,
        );
    }

    assert!(
        *result_count.lock().unwrap() > 0,
        "GROUP BY query should return results"
    );
}
