import {
  AuditRecord,
  BenchmarkSuiteReport,
  ChartData,
  ChartDefinition,
  ColumnMeta,
  LineageReport,
  NamedSource,
  PivotDefinition,
  PivotResult,
  QueryResult,
  StagedPatch,
  TableSchema,
  ViewportResponse,
  WorksheetTable,
  WritebackReviewItem,
} from './types';

const BASE_URL = '';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE_URL}${url}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });
  if (!res.ok) {
    const errText = await res.text();
    throw new Error(errText || `HTTP error ${res.status}`);
  }
  return res.json();
}

export const api = {
  getWorkbook: () => fetchJson<any>('/api/workbook'),
  newWorkbook: () => fetchJson<{ status: string }>('/api/workbook/new', { method: 'POST' }),
  openWorkbook: (path: string) => fetchJson<any>('/api/workbook/open', { method: 'POST', body: JSON.stringify({ path }) }),
  saveWorkbook: (path: string) => fetchJson<any>('/api/workbook/save', { method: 'POST', body: JSON.stringify({ path }) }),

  getConnections: () => fetchJson<{ name: string; kind: string; path: string }[]>('/api/connections'),
  connectDb: (name: string, kind: string, path: string) =>
    fetchJson<any>('/api/connections', {
      method: 'POST',
      body: JSON.stringify({ name, kind, path }),
    }),
  getTables: (connName: string) => fetchJson<string[]>(`/api/connections/${encodeURIComponent(connName)}/tables`),
  getTableSchema: (connName: string, tableName: string) =>
    fetchJson<TableSchema>(`/api/connections/${encodeURIComponent(connName)}/tables/${encodeURIComponent(tableName)}/schema`),

  executeQuery: (connection_name: string, sql: string) =>
    fetchJson<QueryResult>('/api/query/execute', {
      method: 'POST',
      body: JSON.stringify({ connection_name, sql }),
    }),
  analyzeQuery: (connection_name: string, sql: string) =>
    fetchJson<LineageReport>('/api/query/analyze', {
      method: 'POST',
      body: JSON.stringify({ connection_name, sql }),
    }),

  getSources: () => fetchJson<NamedSource[]>('/api/sources'),
  createSource: (name: string, connection_name: string, sql: string) =>
    fetchJson<NamedSource>('/api/sources', {
      method: 'POST',
      body: JSON.stringify({ name, connection_name, sql }),
    }),
  refreshSource: (name: string) =>
    fetchJson<any>(`/api/sources/${encodeURIComponent(name)}/refresh`, { method: 'POST' }),

  insertTable: (sheet_name: string, table_name: string, source_name: string, row: number, col: number) =>
    fetchJson<WorksheetTable>('/api/tables/insert', {
      method: 'POST',
      body: JSON.stringify({ sheet_name, table_name, source_name, row, col }),
    }),

  setCell: (sheet_name: string, row: number, col: number, raw_input: string) =>
    fetchJson<{ status: string }>('/api/cells/set', {
      method: 'POST',
      body: JSON.stringify({ sheet_name, row, col, raw_input }),
    }),
  getViewport: (sheet_name: string, start_row: number, start_col: number, end_row: number, end_col: number, overscan = 5) =>
    fetchJson<ViewportResponse>('/api/cells/viewport', {
      method: 'POST',
      body: JSON.stringify({ sheet_name, start_row, start_col, end_row, end_col, overscan }),
    }),

  getStaged: () => fetchJson<StagedPatch[]>('/api/staged'),
  stageEdit: (data: {
    connection_name: string;
    table_name: string;
    pk_columns: string[];
    pk_values: any[];
    column_name: string;
    original_value: any;
    proposed_value: any;
  }) => fetchJson<{ status: string; id: string }>('/api/staged', { method: 'POST', body: JSON.stringify(data) }),
  discardStaged: (id: string) => fetchJson<{ status: string }>(`/api/staged/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  discardAllStaged: () => fetchJson<{ status: string }>('/api/staged', { method: 'DELETE' }),

  reviewWriteback: () => fetchJson<WritebackReviewItem[]>('/api/writeback/review'),
  applyWriteback: (connection_name: string) =>
    fetchJson<AuditRecord>('/api/writeback/apply', {
      method: 'POST',
      body: JSON.stringify({ connection_name }),
    }),
  getAuditLog: () => fetchJson<AuditRecord[]>('/api/audit'),

  getPivots: () => fetchJson<PivotDefinition[]>('/api/pivots'),
  savePivot: (def: PivotDefinition) =>
    fetchJson<any>('/api/pivots', { method: 'POST', body: JSON.stringify(def) }),
  getPivotData: (name: string) => fetchJson<PivotResult>(`/api/pivots/${encodeURIComponent(name)}/data`),

  getCharts: () => fetchJson<ChartDefinition[]>('/api/charts'),
  saveChart: (def: ChartDefinition) =>
    fetchJson<any>('/api/charts', { method: 'POST', body: JSON.stringify(def) }),
  getChartData: (id: string) => fetchJson<ChartData>(`/api/charts/${encodeURIComponent(id)}/data`),

  runBenchmarks: () => fetchJson<BenchmarkSuiteReport>('/api/benchmark/run', { method: 'POST' }),
};
