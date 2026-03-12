use std::path::Path;
use surrealdb::engine::any;
use surrealdb::engine::local::SurrealKv;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

#[derive(Debug, SurrealValue)]
struct TestRecord {
    content: String,
}

const BASE_SPACE: &str = "/tmp/surrealdb path test";
const BASE_ENCODED: &str = "/tmp/surrealdb%20path%20test";

fn walk_and_report(label: &str) {
    println!("\n  Filesystem check for {label}:");
    for base in [BASE_SPACE, BASE_ENCODED] {
        let p = Path::new(base);
        if p.exists() {
            println!("    EXISTS: {base}");
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    println!("      -> {}", entry.path().display());
                }
            }
        } else {
            println!("    MISSING: {base}");
        }
    }
}

fn check_pass_fail(label: &str) -> bool {
    let encoded_path = Path::new(BASE_ENCODED);
    if encoded_path.exists() {
        println!("  RESULT: FAIL — {label} created a percent-encoded directory!");
        println!("    Found: {BASE_ENCODED}");
        true // bug reproduced
    } else {
        let space_path = Path::new(BASE_SPACE);
        if space_path.exists() {
            println!("  RESULT: PASS — {label} correctly used literal spaces in path.");
        } else {
            println!("  RESULT: UNCLEAR — neither path exists. Connection may have failed.");
        }
        false
    }
}

fn cleanup() {
    for base in [BASE_SPACE, BASE_ENCODED] {
        let p = Path::new(base);
        if p.exists() {
            if let Err(e) = std::fs::remove_dir_all(p) {
                eprintln!("  Warning: failed to clean up {base}: {e}");
            } else {
                println!("  Cleaned up: {base}");
            }
        }
    }
}

async fn scenario_1_any_connect_with_space() -> Result<bool, Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(60));
    println!("SCENARIO 1: any::connect() with space in URL");
    println!("  Endpoint: surrealkv:///tmp/surrealdb path test/via-any/db");
    println!("{}", "=".repeat(60));

    let db = any::connect("surrealkv:///tmp/surrealdb path test/via-any/db").await?;
    println!("  Connected OK.");

    db.use_ns("test").use_db("test").await?;
    println!("  use_ns/use_db OK.");

    let _created: Option<TestRecord> = db
        .create(("test", "1"))
        .content(TestRecord {
            content: "hello".into(),
        })
        .await?;
    println!("  Insert OK.");

    let result: Option<TestRecord> = db.select(("test", "1")).await?;
    match &result {
        Some(r) => println!("  Readback OK: content = {:?}", r.content),
        None => println!("  Readback returned None!"),
    }

    drop(db);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    walk_and_report("Scenario 1 (any::connect with space)");
    let bug = check_pass_fail("Scenario 1");

    cleanup();
    Ok(bug)
}

async fn scenario_2_typed_engine() -> Result<bool, Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(60));
    println!("SCENARIO 2: Surreal::new::<SurrealKv>() with plain path (control)");
    println!("  Path: /tmp/surrealdb path test/via-typed/db");
    println!("{}", "=".repeat(60));

    let db = Surreal::new::<SurrealKv>("/tmp/surrealdb path test/via-typed/db").await?;
    println!("  Connected OK.");

    db.use_ns("test").use_db("test").await?;
    println!("  use_ns/use_db OK.");

    let _created: Option<TestRecord> = db
        .create(("test", "1"))
        .content(TestRecord {
            content: "hello".into(),
        })
        .await?;
    println!("  Insert OK.");

    let result: Option<TestRecord> = db.select(("test", "1")).await?;
    match &result {
        Some(r) => println!("  Readback OK: content = {:?}", r.content),
        None => println!("  Readback returned None!"),
    }

    drop(db);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    walk_and_report("Scenario 2 (typed SurrealKv)");
    let bug = check_pass_fail("Scenario 2");

    cleanup();
    Ok(bug)
}

async fn scenario_3_preencoded() -> Result<bool, Box<dyn std::error::Error>> {
    println!("\n{}", "=".repeat(60));
    println!("SCENARIO 3: any::connect() with pre-encoded %20 in URL");
    println!("  Endpoint: surrealkv:///tmp/surrealdb%20path%20test/via-preencoded/db");
    println!("{}", "=".repeat(60));

    let db =
        any::connect("surrealkv:///tmp/surrealdb%20path%20test/via-preencoded/db").await?;
    println!("  Connected OK.");

    db.use_ns("test").use_db("test").await?;
    println!("  use_ns/use_db OK.");

    let _created: Option<TestRecord> = db
        .create(("test", "1"))
        .content(TestRecord {
            content: "hello".into(),
        })
        .await?;
    println!("  Insert OK.");

    let result: Option<TestRecord> = db.select(("test", "1")).await?;
    match &result {
        Some(r) => println!("  Readback OK: content = {:?}", r.content),
        None => println!("  Readback returned None!"),
    }

    drop(db);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    walk_and_report("Scenario 3 (pre-encoded %20)");
    let bug = check_pass_fail("Scenario 3");

    cleanup();
    Ok(bug)
}

#[tokio::main]
async fn main() {
    println!("SurrealDB Path Encoding Bug Reproducer");
    println!("=======================================\n");
    println!("Testing whether any::connect() percent-encodes filesystem paths.\n");

    let mut results = Vec::new();

    // Scenario 1
    match scenario_1_any_connect_with_space().await {
        Ok(bug) => results.push(("Scenario 1 (any::connect + space)", bug)),
        Err(e) => {
            eprintln!("  Scenario 1 ERROR: {e}");
            results.push(("Scenario 1 (any::connect + space)", false));
            cleanup();
        }
    }

    // Scenario 2
    match scenario_2_typed_engine().await {
        Ok(bug) => results.push(("Scenario 2 (typed SurrealKv)", bug)),
        Err(e) => {
            eprintln!("  Scenario 2 ERROR: {e}");
            results.push(("Scenario 2 (typed SurrealKv)", false));
            cleanup();
        }
    }

    // Scenario 3
    match scenario_3_preencoded().await {
        Ok(bug) => results.push(("Scenario 3 (pre-encoded %20)", bug)),
        Err(e) => {
            eprintln!("  Scenario 3 ERROR: {e}");
            results.push(("Scenario 3 (pre-encoded %20)", false));
            cleanup();
        }
    }

    // Summary
    println!("\n\n=======================================");
    println!("SUMMARY");
    println!("=======================================");
    for (name, bug) in &results {
        let status = if *bug { "BUG REPRODUCED" } else { "OK / no bug" };
        println!("  {name}: {status}");
    }

    let any_bug = results.iter().any(|(_, b)| *b);
    println!();
    if any_bug {
        println!("CONCLUSION: The percent-encoding bug exists in the Rust surrealdb crate.");
    } else {
        println!("CONCLUSION: No percent-encoding bug detected. The fix works!");
    }
}
