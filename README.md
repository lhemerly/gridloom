# Gridloom 🧵

[![CI](https://github.com/lhemerly/gridloom/actions/workflows/ci.yml/badge.svg)](https://github.com/lhemerly/gridloom/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.95%2B-orange.svg)](https://www.rust-lang.org/)
[![SQLite](https://img.shields.io/badge/SQLite-Native-003B57.svg)](https://www.sqlite.org/)
[![DuckDB](https://img.shields.io/badge/DuckDB-Native-FFF000.svg)](https://duckdb.org/)

> **Local-first desktop workbook combining SQL-connected data with an Excel-like worksheet.**

Connect an existing **SQLite** or **DuckDB** database, name a SQL query result as a worksheet data source, and work with it alongside ordinary cells, formulas, pivots, and charts.

The database remains in its original format. Editing an eligible source-backed table creates staged changes; **nothing writes back until the user reviews and confirms them**.

---

## Key Features

- 🔌 **Native SQLite & DuckDB**: Direct connection without foreign server daemons or cloud accounts.
- 🛡️ **Read-Only Capability Boundary**: Mutating queries (`INSERT`, `UPDATE`, `DELETE`, `DROP`, `ATTACH`, extension loads) are strictly blocked at the AST parser security layer.
- 📐 **AST Lineage Analyzer**: Staged writeback requires conservative single-table lineage, direct column mappings, and declared primary keys. Aggregated or joined views are safely marked read-only.
- 📝 **Safe Staged Writeback**: Edits are queued with original and proposed values. The review interface performs NULL-safe optimistic concurrency conflict checks. Applying executes in a single database transaction; conflicts or errors safely roll back the entire changeset without data loss.
- 🧮 **70+ Formula Catalog**: Complete tested implementations across Arithmetic, Statistics, Logic, Text, Dates, Lookup/Arrays, Financial, and SQL formulas (`SQLVALUE`, `SQLTABLE`).
- 🌊 **Dynamic Array Spills**: Excel-style dynamic spilling with automatic result region reservation and `#SPILL!` collision detection.
- 🏷️ **Structured References**: Reference table columns directly with `Table[Column]` and `Table[@Column]` syntax.
- 📊 **Pivots & Visual Charts**: Interactive pivot table generation (Sum, Count, Average, Min, Max) and responsive SVG charts (Column, Bar, Line, Pie, Scatter).
- ⚡ **High-Performance Virtualized Grid**: Renders a bounded viewport; query results are currently fully materialized in memory.
- 💾 **Atomic File Package**: Workbooks save connections, layouts, formulas, and pending changes with atomic replacement (`.gridloom`).

---

## Architecture Overview

```
                      ┌────────────────────────────────────────┐
                      │    Gridloom Desktop App (Wry / Tao)    │
                      │                   OR                   │
                      │    Modern Browser (React 19 + Vite)    │
                      └──────────────────┬─────────────────────┘
                                         │ Viewport & REST API
                                         ▼
                      ┌────────────────────────────────────────┐
                      │     gridloom-server (Axum 0.8)         │
                      │   - Viewport virtual slicing           │
                      │   - Staged writeback review endpoints  │
                      │   - Automated benchmark runner         │
                      └──────────────────┬─────────────────────┘
                                         │
                                         ▼
                      ┌────────────────────────────────────────┐
                      │       gridloom-core (Rust 2021)        │
                      │   - Exact decimal arithmetic           │
                      │   - Pratt AST formula parser           │
                      │   - Kahn's DAG recalculation engine    │
                      │   - Conservative SQL lineage analyzer  │
                      │   - Optimistic concurrency OCC engine  │
                      │   - Atomic .gridloom persistence       │
                      └──────────────┬──────────────────┬──────┘
                                     │                  │
                                     ▼                  ▼
                         ┌─────────────────┐  ┌─────────────────┐
                         │  Native SQLite  │  │  Native DuckDB  │
                         └─────────────────┘  └─────────────────┘
```

---

## Core Workflow

1. **Connect Database**: Open an existing SQLite or DuckDB file, or launch with bundled sample databases (`fixtures/sample_sales.sqlite` and `fixtures/sample_analytics.duckdb`).
2. **Compose SQL**: Run read-only queries with live writeback eligibility checks.
3. **Insert Table**: Name the source (e.g. `Invoices`) and insert it as an anchor-positioned worksheet table.
4. **Calculations & Structured References**: Write formulas outside occupied table bounds:
   ```excel
   =SUM(Invoices[Amount])
   =AVERAGEIFS(Invoices[Amount], Invoices[Status], "Paid")
   =SQLVALUE("SalesDB", "SELECT SUM(amount) FROM invoices WHERE customer_name = ?", A2)
   ```
5. **Stage Edits**: Double-click any qualified row to edit amounts or statuses. Changes are highlighted in amber and staged locally without touching the database.
6. **Review & Writeback**: Inspect the pending changeset in the **Review Edits** drawer. Live conflict detection verifies database rows haven't changed externally. Confirm to execute a single atomic write transaction.

---

## Formula Catalog Contract

Gridloom guarantees exact arithmetic precision (`rust_decimal::Decimal`), distinct SQL `NULL` semantics (separate from blank cells and zero), and Excel-compatible coercion rules.

| Family | Supported Functions |
|---|---|
| **Arithmetic** | `SUM`, `SUMIF`, `SUMIFS`, `SUMPRODUCT`, `PRODUCT`, `ABS`, `SIGN`, `MOD`, `POWER`, `SQRT`, `ROUND`, `ROUNDUP`, `ROUNDDOWN`, `INT`, `TRUNC`, `CEILING.MATH`, `FLOOR.MATH` |
| **Statistics** | `AVERAGE`, `AVERAGEIF`, `AVERAGEIFS`, `COUNT`, `COUNTA`, `COUNTBLANK`, `COUNTIF`, `COUNTIFS`, `MIN`, `MAX`, `MEDIAN`, `STDEV.S`, `STDEV.P`, `VAR.S`, `VAR.P` |
| **Logic & Inspection** | `IF`, `IFS`, `AND`, `OR`, `NOT`, `IFERROR`, `IFNA`, `ISBLANK`, `ISNUMBER`, `ISTEXT`, `ISERROR`, `ISNA` |
| **Text** | `CONCAT`, `TEXTJOIN`, `LEFT`, `RIGHT`, `MID`, `LEN`, `TRIM`, `CLEAN`, `UPPER`, `LOWER`, `SUBSTITUTE`, `REPLACE`, `FIND`, `SEARCH`, `VALUE`, `TEXT` |
| **Dates** | `DATE`, `YEAR`, `MONTH`, `DAY`, `TODAY`, `NOW`, `EDATE`, `EOMONTH`, `DAYS`, `WEEKDAY`, `NETWORKDAYS`, `WORKDAY` |
| **Lookup & Arrays** | `XLOOKUP`, `VLOOKUP`, `INDEX`, `MATCH`, `FILTER`, `SORT`, `UNIQUE`, `SEQUENCE`, `TRANSPOSE` |
| **Financial** | `PMT`, `PV`, `FV`, `NPV`, `IRR` |
| **SQL First-Class** | `SQLVALUE(connection, query, ...params)`, `SQLTABLE(connection, query, ...params)` |

---

## Safe Staged Writeback Guarantee

```
  ┌──────────────┐      Edit Cell       ┌────────────────────────┐
  │  Worksheet   │ ───────────────────► │   Staged Patch Store   │
  │  Table Cell  │                      │ (Key, OldVal, Proposed)│
  └──────────────┘                      └───────────┬────────────┘
                                                    │
                                        Open Review Modal
                                                    │
                                                    ▼
                                        ┌────────────────────────┐
                                        │ Live Conflict Check    │
                                        │ (Reread DB base rows)  │
                                        └───────────┬────────────┘
                                                    │
                                          Confirm & Apply
                                                    │
                                                    ▼
                       ┌─────────────────────────────────────────────────────────┐
                       │               Single Atomic DB Transaction              │
                       │  1. Check NULL-safe equality: current == original       │
                       │  2. If any mismatch or deleted row: ROLLBACK ALL        │
                       │  3. Execute UPDATE ... WHERE id = ?                     │
                       │  4. Write permanent Audit Trail Record                  │
                       │  5. Commit Transaction                                  │
                       └─────────────────────────────────────────────────────────┘
```

---

## V1 Performance Acceptance Protocol

The benchmark runner measures four smoke-test workloads: 100 formula edits, a DuckDB aggregation over 100,000 generated rows, 200 scalar XLOOKUP evaluations over 1,000-element arrays, and a 1,581-cell viewport.

Run `cargo run -p gridloom-server -- --bench` for measurements on your machine. Throughput counts actual operations; the formula workload includes recalculation and the lookup workload includes argument-array clones. Query APIs fully materialize results; viewport rendering does not bound query memory or provide database streaming.

Correctness checks validate the final chain value, aggregate sums and counts, lookup answers, and viewport size. Resource smoke-test budgets are 30 seconds per workload and 1 GiB process peak RSS. Linux reads `VmHWM` from `/proc/self/status`; this is a cumulative process high-water mark, not per-workload allocations. Other hosts report unmeasured memory and an unavailable resource gate. The CLI exits unsuccessfully when either gate fails or resource measurement is unavailable. These smoke tests do not establish production-scale V1 acceptance.

---

## Getting Started

### Prerequisites

- [Rust 1.88+](https://www.rust-lang.org/tools/install)
- [Node.js 20+](https://nodejs.org/)
- WebKitGTK (on Linux for native window shell: `sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev`)

### 1. Build and Run the Complete Application

```bash
# 1. Build the React frontend
cd frontend
npm install
npm run build
cd ..

# 2. Run the Gridloom Desktop application (or server)
cargo run -p gridloom-server
```
Visit `http://127.0.0.1:3001` in your browser, or run the native desktop shell:
```bash
cargo run -p gridloom-desktop
```

### 2. Run the Test Suites

```bash
# Run all core, parser, formula, lineage, and writeback integration tests
cargo test --workspace
```

### 3. Run the Performance Acceptance Suite

```bash
cargo run -p gridloom-server -- --bench
```

---

## License

This project is dual-licensed under either the [MIT License](LICENSE) or Apache 2.0 at your option.
