export type CellValue =
  | { type: 'Blank' | 'EmptyText' | 'Null' }
  | { type: 'Text' | 'Date' | 'Timestamp' | 'Decimal'; value: string }
  | { type: 'Integer' | 'Float'; value: number }
  | { type: 'Boolean'; value: boolean }
  | { type: 'Array'; value: CellValue[][] }
  | { type: 'Error'; value: unknown };

export interface WorkbookSummary {
  active_sheet: string;
  sheets: { name: string; tables: WorksheetTable[] }[];
  connections: string[];
  sources: string[];
  staged_patches_count: number;
  file_path: string | null;
}

export interface CellCoord {
  row: number;
  col: number;
}

export interface CellRenderData {
  coord: CellCoord;
  display_text: string;
  raw_value: CellValue;
  formula?: string;
  number_format?: string;
  is_staged: boolean;
  is_spill: boolean;
  is_error: boolean;
  table_name?: string;
  is_table_header: boolean;
}

export interface ViewportResponse {
  sheet_name: string;
  start_row: number;
  start_col: number;
  end_row: number;
  end_col: number;
  cells: Record<string, CellRenderData>;
  total_rows: number;
  total_cols: number;
  generation: number;
}

export interface ColumnMeta {
  name: string;
  data_type: string;
  is_primary_key: boolean;
  is_nullable: boolean;
}

export interface TableSchema {
  table_name: string;
  columns: ColumnMeta[];
  primary_keys: string[];
}

export interface QueryResult {
  columns: ColumnMeta[];
  rows: CellValue[][];
  total_rows: number;
}

export interface LineageReport {
  is_editable: boolean;
  base_table?: string;
  primary_keys: string[];
  editable_columns: string[];
  read_only_columns: string[];
  read_only_reasons: string[];
}

export interface NamedSource {
  name: string;
  connection_name: string;
  sql: string;
  last_result?: QueryResult;
  lineage?: LineageReport;
}

export interface WorksheetTable {
  name: string;
  source_name: string;
  anchor: CellCoord;
  columns: string[];
}

export interface StagedPatch {
  id: string;
  connection_name: string;
  table_name: string;
  pk_columns: string[];
  pk_values: CellValue[];
  column_name: string;
  original_value: CellValue;
  proposed_value: CellValue;
  staged_at: string;
}

export interface WritebackReviewItem {
  patch: StagedPatch;
  conflict_status: 'NoConflict' | 'RowDeletedExternally' | { CheckFailed: { message: string } } | { RowModifiedExternally: { current_value: any } };
  can_apply: boolean;
}

export interface AuditRecord {
  id: string;
  connection_name: string;
  table_name: string;
  applied_at: string;
  patch_count: number;
  status: 'AppliedSuccessfully' | 'RolledBackConflict' | 'RolledBackError';
  error_message?: string;
  patches: StagedPatch[];
}

export type PivotAggregation = 'Count' | 'Sum' | 'Average' | 'Min' | 'Max';

export interface PivotDefinition {
  name: string;
  source_table: string;
  row_fields: string[];
  col_fields: string[];
  value_field: string;
  aggregation: PivotAggregation;
}

export interface PivotResult {
  headers: string[];
  rows: CellValue[][];
}

export type ChartType = 'Line' | 'Bar' | 'Column' | 'Scatter' | 'Pie';

export interface ChartSeriesBinding {
  name: string;
  value_column: string;
}

export interface ChartDefinition {
  id: string;
  title: string;
  chart_type: ChartType;
  source_table: string;
  category_column: string;
  series: ChartSeriesBinding[];
}

export interface ChartDataSeries {
  name: string;
  values: number[];
}

export interface ChartData {
  title: string;
  chart_type: ChartType;
  categories: string[];
  series: ChartDataSeries[];
}

export interface BenchmarkResult {
  workload_name: string;
  operations: number;
  elapsed_ms: number;
  throughput_ops_per_sec: number;
  memory_peak_rss_kb: number | null;
  status: string;
  notes: string;
}

export interface BenchmarkSuiteReport {
  total_workloads: number;
  total_elapsed_ms: number;
  results: BenchmarkResult[];
  passed_correctness_gate: boolean;
  passed_resource_gate: boolean | null;
}
