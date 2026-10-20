use gridloom_core::value::{CellCoord, Value};
use gridloom_core::workbook::Workbook;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub workload_name: String,
    pub operations: usize,
    pub elapsed_ms: f64,
    pub throughput_ops_per_sec: f64,
    /// Process peak resident memory at the end of this workload, not allocated bytes.
    pub memory_peak_rss_kb: Option<usize>,
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuiteReport {
    pub total_workloads: usize,
    pub total_elapsed_ms: f64,
    pub results: Vec<BenchmarkResult>,
    pub passed_correctness_gate: bool,
    pub passed_resource_gate: Option<bool>,
}

pub fn run_performance_benchmarks() -> BenchmarkSuiteReport {
    let start_all = Instant::now();
    let mut results = Vec::new();

    // 1. One hundred edits, each recalculating the current dependency chain.
    {
        let mut wb = Workbook::new();
        wb.set_cell("Sheet1", CellCoord::new(0, 0), "1".to_string())
            .unwrap();

        let t0 = Instant::now();
        let ops = 100;
        for i in 1..=ops {
            let prev_coord = CellCoord::new(i as u32 - 1, 0);
            let cur_coord = CellCoord::new(i as u32, 0);
            wb.set_cell("Sheet1", cur_coord, format!("={} + 1", prev_coord.to_a1()))
                .unwrap();
        }
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        let last_val = wb
            .sheets
            .get("Sheet1")
            .unwrap()
            .get_cell(&CellCoord::new(ops as u32, 0))
            .unwrap()
            .evaluated_value
            .clone();

        results.push(BenchmarkResult {
            workload_name: "Small Interactive Dependency Chain (100 formulas)".to_string(),
            operations: ops,
            elapsed_ms: elapsed,
            throughput_ops_per_sec: (ops as f64) / (elapsed / 1000.0),
            memory_peak_rss_kb: peak_rss_kb(),
            status: if last_val == Value::Integer((ops + 1) as i64) {
                "PASSED".to_string()
            } else {
                "FAILED".to_string()
            },
            notes: format!("Final chain value: {}", last_val),
        });
    }

    // 2. In-memory analytical DuckDB scan & aggregation (100,000 generated records)
    {
        let mut wb = Workbook::new();
        wb.add_duckdb_memory("BenchDuck").unwrap();

        let conn = wb.connections.get("BenchDuck").unwrap();
        let query_sql = "SELECT CASE WHEN range % 2 = 0 THEN 'A' ELSE 'B' END AS category, SUM(range * 1.5), COUNT(*) FROM range(100000) GROUP BY category ORDER BY category";

        let t0 = Instant::now();
        let q_res = conn.execute_read_query(query_sql, &[]).unwrap();
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

        results.push(BenchmarkResult {
            workload_name: "DuckDB Analytical Aggregate (100k rows scan & group-by)".to_string(),
            operations: 100000,
            elapsed_ms: elapsed,
            throughput_ops_per_sec: 100000.0 / (elapsed / 1000.0),
            memory_peak_rss_kb: peak_rss_kb(),
            status: if q_res.rows
                == vec![
                    vec![
                        Value::Text("A".into()),
                        Value::Integer(3_749_925_000),
                        Value::Integer(50_000),
                    ],
                    vec![
                        Value::Text("B".into()),
                        Value::Integer(3_750_000_000),
                        Value::Integer(50_000),
                    ],
                ] {
                "PASSED".to_string()
            } else {
                "FAILED".to_string()
            },
            notes: format!(
                "Produced {} aggregate group rows in {:.2}ms",
                q_res.rows.len(),
                elapsed
            ),
        });
    }

    // 3. Two hundred scalar XLOOKUP evaluations over 1,000-element arrays.
    {
        let ids: Vec<Vec<Value>> = (1..=1000).map(|i| vec![Value::Integer(i)]).collect();
        let names: Vec<Vec<Value>> = (1..=1000)
            .map(|i| vec![Value::Text(format!("User_{}", i))])
            .collect();

        let t0 = Instant::now();
        let targets: Vec<i64> = (1..=1000).step_by(5).collect();
        let lookups = targets.len();
        let mut correct = true;
        for i in targets {
            let res = gridloom_core::functions::lookup::xlookup(&[
                Value::Integer(i),
                Value::Array(ids.clone()),
                Value::Array(names.clone()),
            ])
            .unwrap();
            if res != Value::Text(format!("User_{}", i)) {
                correct = false;
            }
        }
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

        results.push(BenchmarkResult {
            workload_name: "Scalar XLOOKUP Throughput (1k item array)".to_string(),
            operations: lookups as usize,
            elapsed_ms: elapsed,
            throughput_ops_per_sec: (lookups as f64) / (elapsed / 1000.0),
            memory_peak_rss_kb: peak_rss_kb(),
            status: if correct {
                "PASSED".to_string()
            } else {
                "FAILED".to_string()
            },
            notes: "200 scalar lookups; includes cloning the two argument arrays".to_string(),
        });
    }

    // 4. Viewport slicing with bounded overscan (virtualized rendering simulation)
    {
        let mut wb = Workbook::new();
        // Simulate a sheet with 5,000 populated cells
        for r in 0..100 {
            for c in 0..50 {
                let cell_data = gridloom_core::workbook::CellData::new(
                    CellCoord::new(r, c),
                    format!("{}", r * 100 + c),
                );
                wb.sheets
                    .get_mut("Sheet1")
                    .unwrap()
                    .cells
                    .insert(CellCoord::new(r, c).to_a1(), cell_data);
            }
        }

        let t0 = Instant::now();
        let req = gridloom_core::virtual_view::ViewportRequest {
            sheet_name: "Sheet1".to_string(),
            start_row: 20,
            start_col: 10,
            end_row: 60,
            end_col: 30,
            overscan: 5,
        };
        let vp = wb.get_viewport(&req).unwrap();
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;

        results.push(BenchmarkResult {
            workload_name: "Virtualized Viewport Slicing (bounded overscan)".to_string(),
            operations: vp.cells.len(),
            elapsed_ms: elapsed,
            throughput_ops_per_sec: (vp.cells.len() as f64) / (elapsed / 1000.0),
            memory_peak_rss_kb: peak_rss_kb(),
            status: if vp.cells.len() == 1581 {
                "PASSED".to_string()
            } else {
                "FAILED".to_string()
            },
            notes: format!(
                "Retrieved {} cells for visible window in {:.2}ms",
                vp.cells.len(),
                elapsed
            ),
        });
    }

    let total_elapsed = start_all.elapsed().as_secs_f64() * 1000.0;
    let all_passed = results.iter().all(|r| r.status == "PASSED");

    let passed_resource_gate = resource_gate(&results);
    BenchmarkSuiteReport {
        total_workloads: results.len(),
        total_elapsed_ms: total_elapsed,
        results,
        passed_correctness_gate: all_passed,
        passed_resource_gate,
    }
}

// Linux reports a cumulative process high-water mark. Unsupported hosts report no measurement.
fn peak_rss_kb() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

pub fn resource_gate(results: &[BenchmarkResult]) -> Option<bool> {
    // Smoke-test budgets, not a claim of production-scale acceptance.
    let mut measured = true;
    for r in results {
        if !r.elapsed_ms.is_finite() || r.elapsed_ms > 30_000.0 {
            return Some(false);
        }
        match r.memory_peak_rss_kb {
            Some(kb) if kb > 1_048_576 => return Some(false),
            Some(_) => {}
            None => measured = false,
        }
    }
    if measured && !results.is_empty() {
        Some(true)
    } else {
        None
    }
}

impl BenchmarkSuiteReport {
    pub fn ensure_passed(&self) -> anyhow::Result<()> {
        if !self.passed_correctness_gate {
            anyhow::bail!("Benchmark correctness gate failed");
        }
        match self.passed_resource_gate {
            Some(true) => Ok(()),
            Some(false) => anyhow::bail!("Benchmark resource gate failed"),
            None => anyhow::bail!("Benchmark resource gate unavailable: peak RSS was not measured"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_or_unmeasured_gates_cannot_succeed() {
        let mut r = BenchmarkSuiteReport {
            total_workloads: 0,
            total_elapsed_ms: 0.0,
            results: vec![],
            passed_correctness_gate: false,
            passed_resource_gate: Some(true),
        };
        assert!(r.ensure_passed().is_err());
        r.passed_correctness_gate = true;
        r.passed_resource_gate = None;
        assert!(r.ensure_passed().is_err());
        r.passed_resource_gate = Some(false);
        assert!(r.ensure_passed().is_err());
        assert_eq!(resource_gate(&[]), None);
        let mut workload = BenchmarkResult {
            workload_name: "gate test".into(),
            operations: 1,
            elapsed_ms: 1.0,
            throughput_ops_per_sec: 1000.0,
            memory_peak_rss_kb: Some(1024),
            status: "PASSED".into(),
            notes: String::new(),
        };
        assert_eq!(resource_gate(&[workload.clone()]), Some(true));
        workload.memory_peak_rss_kb = Some(1_048_577);
        assert_eq!(resource_gate(&[workload.clone()]), Some(false));
        workload.memory_peak_rss_kb = None;
        assert_eq!(resource_gate(&[workload.clone()]), None);
        workload.elapsed_ms = 30_001.0;
        assert_eq!(resource_gate(&[workload]), Some(false));
    }
}
