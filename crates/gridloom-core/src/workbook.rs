use crate::ast::StructuredRef;
use crate::chart::{ChartData, ChartDataSeries, ChartDefinition};
use crate::db::{DbConnection, EngineKind};
use crate::depgraph::{DepNode, DependencyGraph};
use crate::error::CellError;
use crate::evaluator::evaluate_expr;
use crate::functions::EvaluationContext;
use crate::lineage::analyze_lineage;
use crate::parser::parse_formula;
use crate::pivot::{compute_pivot, PivotDefinition, PivotResult};
use crate::spill::SpillManager;
use crate::table::{NamedSource, WorksheetTable};
use crate::value::{CellCoord, CellRange, Value};
use crate::virtual_view::{CellRenderData, ViewportRequest, ViewportResponse};
use crate::writeback::StagedRepository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    pub coord: CellCoord,
    pub raw_input: String,
    pub formula: Option<String>,
    pub evaluated_value: Value,
    pub number_format: Option<String>,
    pub is_dirty: bool,
}

impl CellData {
    pub fn new(coord: CellCoord, raw_input: String) -> Self {
        let is_formula = raw_input.trim().starts_with('=');
        let formula = if is_formula {
            Some(raw_input.trim().to_string())
        } else {
            None
        };
        let evaluated_value = if !is_formula {
            parse_literal_or_text(&raw_input)
        } else {
            Value::Blank
        };
        Self {
            coord,
            raw_input,
            formula,
            evaluated_value,
            number_format: None,
            is_dirty: is_formula,
        }
    }
}

fn parse_literal_or_text(input: &str) -> Value {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Value::Blank;
    }
    if trimmed.eq_ignore_ascii_case("TRUE") {
        return Value::Boolean(true);
    }
    if trimmed.eq_ignore_ascii_case("FALSE") {
        return Value::Boolean(false);
    }
    if trimmed.eq_ignore_ascii_case("NULL") {
        return Value::Null;
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return Value::Integer(i);
    }
    if let Ok(d) = rust_decimal::Decimal::from_str_exact(trimmed) {
        return Value::Decimal(d);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return Value::Float(f);
    }
    Value::Text(input.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub name: String,
    pub cells: HashMap<String, CellData>,
    pub tables: HashMap<String, WorksheetTable>,
    pub spills: SpillManager,
    pub max_rows: u32,
    pub max_cols: u32,
}

impl Sheet {
    pub fn new(name: String) -> Self {
        Self {
            name,
            cells: HashMap::new(),
            tables: HashMap::new(),
            spills: SpillManager::new(),
            max_rows: 1000,
            max_cols: 100,
        }
    }

    pub fn get_cell(&self, coord: &CellCoord) -> Option<&CellData> {
        self.cells.get(&coord.to_a1())
    }

    pub fn get_cell_mut(&mut self, coord: &CellCoord) -> Option<&mut CellData> {
        self.cells.get_mut(&coord.to_a1())
    }

    pub fn is_cell_occupied(&self, coord: CellCoord) -> bool {
        if let Some(cell) = self.cells.get(&coord.to_a1()) {
            if !cell.evaluated_value.is_blank() {
                return true;
            }
        }
        for table in self.tables.values() {
            if table.range.contains(coord) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub name: String,
    pub kind: EngineKind,
    pub path: String,
}

pub struct Workbook {
    pub sheets: HashMap<String, Sheet>,
    pub active_sheet: String,
    pub connections: HashMap<String, DbConnection>,
    pub connection_configs: HashMap<String, ConnectionConfig>,
    pub named_sources: HashMap<String, NamedSource>,
    pub staged: StagedRepository,
    pub pivots: HashMap<String, PivotDefinition>,
    pub charts: HashMap<String, ChartDefinition>,
    pub dep_graph: DependencyGraph,
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedWorkbook {
    pub sheets: HashMap<String, Sheet>,
    pub active_sheet: String,
    pub connection_configs: HashMap<String, ConnectionConfig>,
    pub named_sources: HashMap<String, NamedSource>,
    pub staged: StagedRepository,
    pub pivots: HashMap<String, PivotDefinition>,
    pub charts: HashMap<String, ChartDefinition>,
}

impl Workbook {
    pub fn new() -> Self {
        let mut sheets = HashMap::new();
        let default_sheet = "Sheet1".to_string();
        sheets.insert(default_sheet.clone(), Sheet::new(default_sheet.clone()));

        Self {
            sheets,
            active_sheet: default_sheet,
            connections: HashMap::new(),
            connection_configs: HashMap::new(),
            named_sources: HashMap::new(),
            staged: StagedRepository::new(),
            pivots: HashMap::new(),
            charts: HashMap::new(),
            dep_graph: DependencyGraph::new(),
            file_path: None,
        }
    }

    pub fn add_sqlite_connection<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<(), CellError> {
        let p_str = path.as_ref().to_string_lossy().to_string();
        let conn = DbConnection::open_sqlite(&path)?;
        self.connections.insert(name.to_string(), conn);
        self.connection_configs.insert(
            name.to_string(),
            ConnectionConfig {
                name: name.to_string(),
                kind: EngineKind::Sqlite,
                path: p_str,
            },
        );
        Ok(())
    }

    pub fn add_sqlite_memory(&mut self, name: &str) -> Result<(), CellError> {
        let conn = DbConnection::open_sqlite_memory()?;
        self.connections.insert(name.to_string(), conn);
        self.connection_configs.insert(
            name.to_string(),
            ConnectionConfig {
                name: name.to_string(),
                kind: EngineKind::Sqlite,
                path: ":memory:".to_string(),
            },
        );
        Ok(())
    }

    pub fn add_duckdb_connection<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<(), CellError> {
        let p_str = path.as_ref().to_string_lossy().to_string();
        let conn = DbConnection::open_duckdb(&path)?;
        self.connections.insert(name.to_string(), conn);
        self.connection_configs.insert(
            name.to_string(),
            ConnectionConfig {
                name: name.to_string(),
                kind: EngineKind::DuckDb,
                path: p_str,
            },
        );
        Ok(())
    }

    pub fn add_duckdb_memory(&mut self, name: &str) -> Result<(), CellError> {
        let conn = DbConnection::open_duckdb_memory()?;
        self.connections.insert(name.to_string(), conn);
        self.connection_configs.insert(
            name.to_string(),
            ConnectionConfig {
                name: name.to_string(),
                kind: EngineKind::DuckDb,
                path: ":memory:".to_string(),
            },
        );
        Ok(())
    }

    pub fn create_named_source(
        &mut self,
        name: &str,
        conn_name: &str,
        sql: &str,
    ) -> Result<NamedSource, CellError> {
        let conn = self
            .connections
            .get(conn_name)
            .ok_or_else(|| CellError::Sql(format!("Connection '{}' not found", conn_name)))?;

        // 1. Check lineage
        let dialect = match conn.kind() {
            EngineKind::Sqlite => "sqlite",
            EngineKind::DuckDb => "duckdb",
        };

        let schema_lookup = |tbl: &str| conn.get_table_schema(tbl).ok();
        let lineage = analyze_lineage(sql, dialect, schema_lookup);

        // 2. Execute read query
        let query_result = conn.execute_read_query(sql, &[])?;

        let source = NamedSource {
            name: name.to_string(),
            connection_name: conn_name.to_string(),
            sql: sql.to_string(),
            parameters: Vec::new(),
            last_result: Some(query_result),
            lineage: Some(lineage),
        };

        self.named_sources.insert(name.to_string(), source.clone());
        Ok(source)
    }

    pub fn insert_table(
        &mut self,
        sheet_name: &str,
        table_name: &str,
        source_name: &str,
        anchor: CellCoord,
    ) -> Result<WorksheetTable, CellError> {
        let source = self
            .named_sources
            .get(source_name)
            .ok_or_else(|| CellError::Sql(format!("Data source '{}' not found", source_name)))?;

        let query_res = source
            .last_result
            .as_ref()
            .ok_or_else(|| CellError::Sql("Data source has no query result".to_string()))?;

        let col_names: Vec<String> = query_res.columns.iter().map(|c| c.name.clone()).collect();
        let row_count = query_res.rows.len() as u32;

        let table = WorksheetTable::new(
            table_name.to_string(),
            source_name.to_string(),
            anchor,
            col_names,
            row_count,
        );

        let sheet = self
            .sheets
            .get_mut(sheet_name)
            .ok_or_else(|| CellError::Ref)?;

        // Collision check: verify target area does not collide with existing occupied cells
        for r in table.range.start_row..=table.range.end_row {
            for c in table.range.start_col..=table.range.end_col {
                let coord = CellCoord::new(r, c);
                if sheet.is_cell_occupied(coord) {
                    return Err(CellError::Spill);
                }
            }
        }

        sheet.tables.insert(table_name.to_string(), table.clone());
        Ok(table)
    }

    pub fn set_cell(&mut self, sheet_name: &str, coord: CellCoord, raw_input: String) -> Result<(), CellError> {
        let sheet = self
            .sheets
            .get_mut(sheet_name)
            .ok_or_else(|| CellError::Ref)?;

        let cell_data = CellData::new(coord, raw_input);
        sheet.cells.insert(coord.to_a1(), cell_data);

        // Invalidate in dependency graph
        let cell_node = DepNode::Cell(coord);
        self.dep_graph.clear_dependencies(&cell_node);

        // Recalculate
        self.recalculate(sheet_name)?;
        Ok(())
    }

    pub fn recalculate(&mut self, sheet_name: &str) -> Result<(), CellError> {
        // Collect dirty cells
        let dirty_coords: Vec<CellCoord> = {
            let sheet = self.sheets.get(sheet_name).ok_or(CellError::Ref)?;
            sheet
                .cells
                .values()
                .filter(|c| c.is_dirty || c.formula.is_some())
                .map(|c| c.coord)
                .collect()
        };

        let dirty_nodes: Vec<DepNode> = dirty_coords.into_iter().map(DepNode::Cell).collect();
        let eval_order = self.dep_graph.get_recalculation_order(&dirty_nodes)?;

        for node in eval_order {
            if let DepNode::Cell(coord) = node {
                self.evaluate_single_cell(sheet_name, coord)?;
            }
        }

        self.dep_graph.generation += 1;
        Ok(())
    }

    fn evaluate_single_cell(&mut self, sheet_name: &str, coord: CellCoord) -> Result<(), CellError> {
        let formula_opt = {
            let sheet = self.sheets.get(sheet_name).ok_or(CellError::Ref)?;
            sheet.get_cell(&coord).and_then(|c| c.formula.clone())
        };

        if let Some(formula) = formula_opt {
            let ast_result = parse_formula(&formula);
            let eval_value = match ast_result {
                Ok(ast) => {
                    // Update dependencies in dep_graph
                    self.dep_graph.clear_dependencies(&DepNode::Cell(coord));
                    self.extract_ast_dependencies(&DepNode::Cell(coord), &ast)?;

                    let ctx = WorkbookEvalContext {
                        workbook: self,
                        sheet_name,
                        current_coord: coord,
                    };
                    evaluate_expr(&ast, &ctx).unwrap_or_else(Value::Error)
                }
                Err(e) => Value::Error(e),
            };

            // Check if returned an array -> dynamic spill!
            let sheet = self.sheets.get_mut(sheet_name).ok_or(CellError::Ref)?;
            if let Value::Array(ref rows) = eval_value {
                let r_count = rows.len();
                let c_count = rows.first().map_or(0, |r| r.len());

                let can_spill = sheet.spills.can_spill(coord, r_count, c_count, |c| {
                    sheet.get_cell(&c).map_or(false, |cell| !cell.evaluated_value.is_blank())
                });

                if can_spill {
                    sheet.spills.register_spill(coord, r_count, c_count);
                    if let Some(c) = sheet.get_cell_mut(&coord) {
                        c.evaluated_value = eval_value;
                        c.is_dirty = false;
                    }
                } else {
                    sheet.spills.clear_anchor(&coord);
                    if let Some(c) = sheet.get_cell_mut(&coord) {
                        c.evaluated_value = Value::Error(CellError::Spill);
                        c.is_dirty = false;
                    }
                }
            } else {
                sheet.spills.clear_anchor(&coord);
                if let Some(c) = sheet.get_cell_mut(&coord) {
                    c.evaluated_value = eval_value;
                    c.is_dirty = false;
                }
            }
        }
        Ok(())
    }

    fn extract_ast_dependencies(&mut self, node: &DepNode, expr: &crate::ast::Expr) -> Result<(), CellError> {
        use crate::ast::Expr;
        match expr {
            Expr::CellRef(coord) => {
                self.dep_graph.add_dependency(node.clone(), DepNode::Cell(*coord))?;
            }
            Expr::RangeRef(range) => {
                for r in range.start_row..=range.end_row {
                    for c in range.start_col..=range.end_col {
                        self.dep_graph.add_dependency(node.clone(), DepNode::Cell(CellCoord::new(r, c)))?;
                    }
                }
            }
            Expr::SpillRef(coord) => {
                self.dep_graph.add_dependency(node.clone(), DepNode::Cell(*coord))?;
            }
            Expr::StructuredRef(sref) => {
                self.dep_graph.add_dependency(node.clone(), DepNode::Table(sref.table_name.clone()))?;
            }
            Expr::Unary { expr, .. } => self.extract_ast_dependencies(node, expr)?,
            Expr::Binary { left, right, .. } => {
                self.extract_ast_dependencies(node, left)?;
                self.extract_ast_dependencies(node, right)?;
            }
            Expr::FunctionCall { args, .. } => {
                for arg in args {
                    self.extract_ast_dependencies(node, arg)?;
                }
            }
            Expr::ArrayLiteral(rows) => {
                for row in rows {
                    for cell in row {
                        self.extract_ast_dependencies(node, cell)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn get_viewport(&self, req: &ViewportRequest) -> Result<ViewportResponse, CellError> {
        let sheet = self.sheets.get(&req.sheet_name).ok_or(CellError::Ref)?;

        let start_row = req.start_row.saturating_sub(req.overscan);
        let end_row = (req.end_row + req.overscan).min(sheet.max_rows);
        let start_col = req.start_col.saturating_sub(req.overscan);
        let end_col = (req.end_col + req.overscan).min(sheet.max_cols);

        let mut cells_data = HashMap::new();

        for r in start_row..=end_row {
            for c in start_col..=end_col {
                let coord = CellCoord::new(r, c);
                let a1 = coord.to_a1();

                // 1. Check if cell is in a worksheet table
                let mut table_hit = None;
                for table in sheet.tables.values() {
                    if table.range.contains(coord) {
                        let is_header = coord.row == table.anchor.row;
                        table_hit = Some((table, is_header));
                        break;
                    }
                }

                if let Some((table, is_header)) = table_hit {
                    let col_offset = (coord.col - table.anchor.col) as usize;
                    if is_header {
                        let col_name = table.columns.get(col_offset).cloned().unwrap_or_default();
                        cells_data.insert(
                            a1.clone(),
                            CellRenderData {
                                coord,
                                display_text: col_name.clone(),
                                raw_value: Value::Text(col_name),
                                formula: None,
                                number_format: None,
                                is_staged: false,
                                is_spill: false,
                                is_error: false,
                                table_name: Some(table.name.clone()),
                                is_table_header: true,
                            },
                        );
                        continue;
                    } else {
                        // Data row from source
                        let row_offset = (coord.row - table.anchor.row - 1) as usize;
                        if let Some(source) = self.named_sources.get(&table.source_name) {
                            if let Some(res) = &source.last_result {
                                if let Some(row_data) = res.rows.get(row_offset) {
                                    let mut val = row_data.get(col_offset).cloned().unwrap_or(Value::Blank);
                                    let col_name = table.columns.get(col_offset).cloned().unwrap_or_default();

                                    // Check staged overlay
                                    let mut is_staged = false;
                                    for patch in self.staged.patches.values() {
                                        if patch.table_name == table.source_name
                                            && patch.column_name == col_name
                                        {
                                            val = patch.proposed_value.clone();
                                            is_staged = true;
                                            break;
                                        }
                                    }

                                    cells_data.insert(
                                        a1.clone(),
                                        CellRenderData {
                                            coord,
                                            display_text: val.to_string_repr(),
                                            raw_value: val,
                                            formula: None,
                                            number_format: None,
                                            is_staged,
                                            is_spill: false,
                                            is_error: false,
                                            table_name: Some(table.name.clone()),
                                            is_table_header: false,
                                        },
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }

                // 2. Check dynamic spill cell
                if let Some(spill) = sheet.spills.get_spill_at(&coord) {
                    if coord != spill.anchor {
                        // Spilled cell value
                        let r_offset = (coord.row - spill.anchor.row) as usize;
                        let c_offset = (coord.col - spill.anchor.col) as usize;
                        if let Some(anchor_cell) = sheet.get_cell(&spill.anchor) {
                            if let Value::Array(ref rows) = anchor_cell.evaluated_value {
                                let val = rows
                                    .get(r_offset)
                                    .and_then(|row| row.get(c_offset))
                                    .cloned()
                                    .unwrap_or(Value::Blank);

                                cells_data.insert(
                                    a1.clone(),
                                    CellRenderData {
                                        coord,
                                        display_text: val.to_string_repr(),
                                        raw_value: val,
                                        formula: None,
                                        number_format: None,
                                        is_staged: false,
                                        is_spill: true,
                                        is_error: false,
                                        table_name: None,
                                        is_table_header: false,
                                    },
                                );
                                continue;
                            }
                        }
                    }
                }

                // 3. Regular cell
                if let Some(cell) = sheet.get_cell(&coord) {
                    let is_err = cell.evaluated_value.is_error();
                    cells_data.insert(
                        a1,
                        CellRenderData {
                            coord,
                            display_text: cell.evaluated_value.to_string_repr(),
                            raw_value: cell.evaluated_value.clone(),
                            formula: cell.formula.clone(),
                            number_format: cell.number_format.clone(),
                            is_staged: false,
                            is_spill: false,
                            is_error: is_err,
                            table_name: None,
                            is_table_header: false,
                        },
                    );
                }
            }
        }

        Ok(ViewportResponse {
            sheet_name: req.sheet_name.clone(),
            start_row,
            start_col,
            end_row,
            end_col,
            cells: cells_data,
            total_rows: sheet.max_rows,
            total_cols: sheet.max_cols,
            generation: self.dep_graph.generation,
        })
    }

    pub fn compute_pivot_result(&self, name: &str) -> Result<PivotResult, CellError> {
        let pivot_def = self
            .pivots
            .get(name)
            .ok_or_else(|| CellError::Name)?;

        let source = self
            .named_sources
            .get(&pivot_def.source_table)
            .ok_or_else(|| CellError::Name)?;

        let res = source
            .last_result
            .as_ref()
            .ok_or_else(|| CellError::Sql("No source query result".to_string()))?;

        let col_names: Vec<String> = res.columns.iter().map(|c| c.name.clone()).collect();
        compute_pivot(pivot_def, &col_names, &res.rows)
    }

    pub fn compute_chart_data(&self, id: &str) -> Result<ChartData, CellError> {
        let chart_def = self
            .charts
            .get(id)
            .ok_or_else(|| CellError::Name)?;

        let source = self
            .named_sources
            .get(&chart_def.source_table)
            .ok_or_else(|| CellError::Name)?;

        let res = source
            .last_result
            .as_ref()
            .ok_or_else(|| CellError::Sql("No source query result".to_string()))?;

        let cat_col_idx = res
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(&chart_def.category_column))
            .ok_or(CellError::Value)?;

        let mut categories = Vec::new();
        for r in &res.rows {
            categories.push(r.get(cat_col_idx).map_or(String::new(), |v| v.to_string_repr()));
        }

        let mut series_data = Vec::new();
        for s in &chart_def.series {
            let val_col_idx = res
                .columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(&s.value_column))
                .ok_or(CellError::Value)?;

            let mut values = Vec::new();
            for r in &res.rows {
                let cell = r.get(val_col_idx).unwrap_or(&Value::Blank);
                values.push(cell.to_f64().unwrap_or(0.0));
            }
            series_data.push(ChartDataSeries {
                name: s.name.clone(),
                values,
            });
        }

        Ok(ChartData {
            title: chart_def.title.clone(),
            chart_type: chart_def.chart_type,
            categories,
            series: series_data,
        })
    }

    pub fn apply_writeback(&mut self, connection_name: &str) -> Result<crate::writeback::AuditRecord, CellError> {
        let conn = self
            .connections
            .get(connection_name)
            .ok_or_else(|| CellError::Sql(format!("Connection '{}' not found", connection_name)))?;
        self.staged.apply_writeback(conn, connection_name)
    }

    /// Atomic file package save
    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), CellError> {
        let p = path.as_ref().to_path_buf();
        let tmp_path = p.with_extension("gridloom.tmp");

        let serialized = SerializedWorkbook {
            sheets: self.sheets.clone(),
            active_sheet: self.active_sheet.clone(),
            connection_configs: self.connection_configs.clone(),
            named_sources: self.named_sources.clone(),
            staged: self.staged.clone(),
            pivots: self.pivots.clone(),
            charts: self.charts.clone(),
        };

        let file = File::create(&tmp_path)
            .map_err(|e| CellError::Sql(format!("Failed to create save file: {}", e)))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &serialized)
            .map_err(|e| CellError::Sql(format!("Failed to serialize workbook: {}", e)))?;

        // Atomic replace
        fs::rename(&tmp_path, &p)
            .map_err(|e| CellError::Sql(format!("Failed to commit atomic save: {}", e)))?;

        self.file_path = Some(p);
        Ok(())
    }

    /// Open and deserialize workbook
    pub fn open_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CellError> {
        let p = path.as_ref().to_path_buf();
        let file = File::open(&p)
            .map_err(|e| CellError::Sql(format!("Failed to open workbook file: {}", e)))?;
        let reader = BufReader::new(file);
        let serialized: SerializedWorkbook = serde_json::from_reader(reader)
            .map_err(|e| CellError::Sql(format!("Corrupt workbook format: {}", e)))?;

        let mut wb = Workbook {
            sheets: serialized.sheets,
            active_sheet: serialized.active_sheet,
            connections: HashMap::new(),
            connection_configs: serialized.connection_configs.clone(),
            named_sources: serialized.named_sources,
            staged: serialized.staged,
            pivots: serialized.pivots,
            charts: serialized.charts,
            dep_graph: DependencyGraph::new(),
            file_path: Some(p),
        };

        // Reconnect active database connections
        for config in serialized.connection_configs.values() {
            match config.kind {
                EngineKind::Sqlite => {
                    if config.path == ":memory:" {
                        let _ = wb.add_sqlite_memory(&config.name);
                    } else if Path::new(&config.path).exists() {
                        let _ = wb.add_sqlite_connection(&config.name, &config.path);
                    }
                }
                EngineKind::DuckDb => {
                    if config.path == ":memory:" {
                        let _ = wb.add_duckdb_memory(&config.name);
                    } else if Path::new(&config.path).exists() {
                        let _ = wb.add_duckdb_connection(&config.name, &config.path);
                    }
                }
            }
        }

        Ok(wb)
    }
}

pub struct WorkbookEvalContext<'a> {
    pub workbook: &'a Workbook,
    pub sheet_name: &'a str,
    pub current_coord: CellCoord,
}

impl<'a> EvaluationContext for WorkbookEvalContext<'a> {
    fn get_cell_value(&self, coord: CellCoord) -> Result<Value, CellError> {
        let sheet = self
            .workbook
            .sheets
            .get(self.sheet_name)
            .ok_or(CellError::Ref)?;

        // 1. Check if cell is in a table
        for table in sheet.tables.values() {
            if table.range.contains(coord) {
                if coord.row == table.anchor.row {
                    let col_offset = (coord.col - table.anchor.col) as usize;
                    let name = table.columns.get(col_offset).cloned().unwrap_or_default();
                    return Ok(Value::Text(name));
                } else {
                    let row_offset = (coord.row - table.anchor.row - 1) as usize;
                    let col_offset = (coord.col - table.anchor.col) as usize;
                    if let Some(source) = self.workbook.named_sources.get(&table.source_name) {
                        if let Some(res) = &source.last_result {
                            if let Some(r) = res.rows.get(row_offset) {
                                return Ok(r.get(col_offset).cloned().unwrap_or(Value::Blank));
                            }
                        }
                    }
                }
            }
        }

        // 2. Regular cell
        if let Some(cell) = sheet.get_cell(&coord) {
            Ok(cell.evaluated_value.clone())
        } else {
            Ok(Value::Blank)
        }
    }

    fn get_range_values(&self, range: CellRange) -> Result<Value, CellError> {
        let mut rows = Vec::new();
        for r in range.start_row..=range.end_row {
            let mut row = Vec::new();
            for c in range.start_col..=range.end_col {
                row.push(self.get_cell_value(CellCoord::new(r, c))?);
            }
            rows.push(row);
        }
        Ok(Value::Array(rows))
    }

    fn get_table_column(&self, sref: &StructuredRef) -> Result<Value, CellError> {
        let sheet = self
            .workbook
            .sheets
            .get(self.sheet_name)
            .ok_or(CellError::Ref)?;

        let table = sheet
            .tables
            .values()
            .find(|t| t.name.eq_ignore_ascii_case(&sref.table_name))
            .ok_or(CellError::Ref)?;

        let range = table.resolve_structured_ref(sref, Some(self.current_coord))?;
        self.get_range_values(range)
    }

    fn execute_sql_value(&self, connection_name: &str, sql: &str, params: &[Value]) -> Result<Value, CellError> {
        let conn = self
            .workbook
            .connections
            .get(connection_name)
            .ok_or_else(|| CellError::Sql(format!("Connection '{}' not found", connection_name)))?;

        let res = conn.execute_read_query(sql, params)?;

        if res.rows.is_empty() {
            // Explicit NULL on 0 rows as specified
            Ok(Value::Null)
        } else if res.rows.len() == 1 && res.columns.len() == 1 {
            Ok(res.rows[0][0].clone())
        } else {
            // Shape error on multiple rows or columns as specified
            Err(CellError::Value)
        }
    }

    fn execute_sql_table(&self, connection_name: &str, sql: &str, params: &[Value]) -> Result<Value, CellError> {
        let conn = self
            .workbook
            .connections
            .get(connection_name)
            .ok_or_else(|| CellError::Sql(format!("Connection '{}' not found", connection_name)))?;

        let res = conn.execute_read_query(sql, params)?;
        Ok(Value::Array(res.rows))
    }
}
