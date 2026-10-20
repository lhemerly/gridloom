import React, { useState, useEffect, useCallback } from 'react';
import { Toolbar } from './components/Toolbar';
import { FormulaBar } from './components/FormulaBar';
import { Grid } from './components/Grid';
import { SchemaBrowserModal } from './components/SchemaBrowserModal';
import { QueryEditorModal } from './components/QueryEditorModal';
import { InsertTableModal } from './components/InsertTableModal';
import { StagedReviewModal } from './components/StagedReviewModal';
import { PivotBuilderModal } from './components/PivotBuilderModal';
import { ChartModal } from './components/ChartModal';
import { AuditLogModal } from './components/AuditLogModal';
import { BenchmarkModal } from './components/BenchmarkModal';
import { api } from './api';
import { ViewportResponse, WorksheetTable } from './types';
import { coordToA1 } from './utils/coords';

export const App: React.FC = () => {
  const [sheetName] = useState('Sheet1');
  const [selectedRow, setSelectedRow] = useState(0);
  const [selectedCol, setSelectedCol] = useState(0);
  const [viewport, setViewport] = useState<ViewportResponse | null>(null);
  const [stagedCount, setStagedCount] = useState(0);
  const [tables, setTables] = useState<WorksheetTable[]>([]);

  // Visible bounds
  const [visibleBounds, setVisibleBounds] = useState({
    startRow: 0,
    startCol: 0,
    endRow: 40,
    endCol: 20,
  });

  // Modal open states
  const [isSchemaBrowserOpen, setIsSchemaBrowserOpen] = useState(false);
  const [isQueryEditorOpen, setIsQueryEditorOpen] = useState(false);
  const [queryInitialConn, setQueryInitialConn] = useState('');
  const [queryInitialSql, setQueryInitialSql] = useState('');
  const [isInsertTableOpen, setIsInsertTableOpen] = useState(false);
  const [isStagedReviewOpen, setIsStagedReviewOpen] = useState(false);
  const [isPivotsOpen, setIsPivotsOpen] = useState(false);
  const [isChartsOpen, setIsChartsOpen] = useState(false);
  const [isAuditLogOpen, setIsAuditLogOpen] = useState(false);
  const [isBenchmarksOpen, setIsBenchmarksOpen] = useState(false);

  // Status message
  const [statusMsg, setStatusMsg] = useState('Ready');

  // Load workbook tables & staged patches count
  const refreshMetadata = useCallback(async () => {
    try {
      const [wb, staged] = await Promise.all([
        api.getWorkbook().catch(() => null),
        api.getStaged().catch(() => []),
      ]);
      if (wb && wb.sheets && wb.sheets[0]) {
        setTables(wb.sheets[0].tables || []);
      }
      setStagedCount(staged.length);
    } catch {
      // ignore
    }
  }, []);

  // Fetch viewport cells
  const fetchViewportData = useCallback(async (startR: number, startC: number, endR: number, endC: number) => {
    try {
      const data = await api.getViewport(sheetName, startR, startC, endR, endC, 5);
      setViewport(data);
    } catch (err: any) {
      console.error('Failed to fetch viewport:', err);
    }
  }, [sheetName]);

  // Initial load
  useEffect(() => {
    fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
    refreshMetadata();
  }, [fetchViewportData, refreshMetadata, visibleBounds]);

  const handleViewportChange = useCallback((startRow: number, startCol: number, endRow: number, endCol: number) => {
    setVisibleBounds({ startRow, startCol, endRow, endCol });
    fetchViewportData(startRow, startCol, endRow, endCol);
  }, [fetchViewportData]);

  const handleCommitCell = async (row: number, col: number, rawInput: string) => {
    try {
      setStatusMsg(`Setting ${coordToA1(row, col)}...`);
      const key = coordToA1(row, col);
      const existing = viewport?.cells[key];

      // Check if this cell belongs to an editable table
      if (existing?.table_name && !existing.is_table_header) {
        // Find table and its source
        const tbl = tables.find((t) => t.name === existing.table_name);
        if (tbl) {
          const sources = await api.getSources();
          const src = sources.find((s) => s.name === tbl.source_name);
          if (src?.lineage?.is_editable && src.last_result) {
            const colIdxInTable = col - tbl.anchor.col;
            const rowIdxInTable = row - tbl.anchor.row - 1; // subtract header
            const targetColName = tbl.columns[colIdxInTable];

            if (targetColName && src.lineage.editable_columns.includes(targetColName)) {
              // Locate primary key column indices in the source result
              const pkCols = src.lineage.primary_keys;
              const pkValues = pkCols.map((pk) => {
                const pkIdx = src.last_result?.columns.findIndex((c) => c.name === pk);
                return pkIdx !== undefined && pkIdx >= 0 ? src.last_result?.rows[rowIdxInTable]?.[pkIdx] : null;
              });

              const origVal = existing.raw_value;
              // Parse proposed value (number vs text)
              let proposedVal: any = rawInput;
              const num = Number(rawInput);
              if (!isNaN(num) && rawInput.trim() !== '') {
                proposedVal = num;
              }

              // Stage edit!
              await api.stageEdit({
                connection_name: src.connection_name,
                table_name: src.lineage.base_table || tbl.source_name,
                pk_columns: pkCols,
                pk_values: pkValues,
                column_name: targetColName,
                original_value: origVal,
                proposed_value: proposedVal,
              });

              setStatusMsg(`Staged edit for ${tbl.name}[${targetColName}]`);
              await refreshMetadata();
              fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
              return;
            }
          }
        }
      }

      // Normal spreadsheet cell edit
      await api.setCell(sheetName, row, col, rawInput);
      setStatusMsg(`Updated ${coordToA1(row, col)}`);
      await refreshMetadata();
      fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
    } catch (err: any) {
      setStatusMsg(`Error: ${err.message}`);
    }
  };

  const handleApplyFormat = async (format: string) => {
    const key = coordToA1(selectedRow, selectedCol);
    const cell = viewport?.cells[key];
    if (cell && cell.formula) {
      await api.setCell(sheetName, selectedRow, selectedCol, cell.formula);
      fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
    }
  };

  const handleNewWorkbook = async () => {
    if (confirm('Create a new workbook? Unsaved changes will be discarded.')) {
      await api.newWorkbook();
      await refreshMetadata();
      fetchViewportData(0, 0, 40, 20);
      setStatusMsg('Created new workbook');
    }
  };

  const handleSaveWorkbook = async () => {
    const path = prompt('Enter save path:', 'my_workbook.gridloom');
    if (path) {
      try {
        await api.saveWorkbook(path);
        setStatusMsg(`Saved workbook to ${path}`);
      } catch (err: any) {
        alert(`Failed to save: ${err.message}`);
      }
    }
  };

  const handleOpenWorkbook = async () => {
    const path = prompt('Enter path of .gridloom file to open:', 'my_workbook.gridloom');
    if (path) {
      try {
        await api.openWorkbook(path);
        await refreshMetadata();
        fetchViewportData(0, 0, 40, 20);
        setStatusMsg(`Opened workbook from ${path}`);
      } catch (err: any) {
        alert(`Failed to open: ${err.message}`);
      }
    }
  };

  const handleRefreshAll = async () => {
    try {
      setStatusMsg('Refreshing all data sources...');
      const sources = await api.getSources();
      for (const s of sources) {
        await api.refreshSource(s.name);
      }
      await refreshMetadata();
      fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
      setStatusMsg('All sources refreshed successfully');
    } catch (err: any) {
      setStatusMsg(`Refresh error: ${err.message}`);
    }
  };

  const openQueryWithTable = (connName: string, tableName: string) => {
    setQueryInitialConn(connName);
    setQueryInitialSql(`SELECT * FROM ${tableName} LIMIT 50`);
    setIsQueryEditorOpen(true);
  };

  const activeKey = coordToA1(selectedRow, selectedCol);
  const activeCellData = viewport?.cells[activeKey];

  return (
    <div className="flex flex-col h-screen w-screen bg-slate-100 overflow-hidden font-sans">
      {/* Top Application Toolbar */}
      <Toolbar
        onOpenSchemaBrowser={() => setIsSchemaBrowserOpen(true)}
        onOpenQueryEditor={() => setIsQueryEditorOpen(true)}
        onOpenInsertTable={() => setIsInsertTableOpen(true)}
        onOpenStagedReview={() => setIsStagedReviewOpen(true)}
        onOpenPivots={() => setIsPivotsOpen(true)}
        onOpenCharts={() => setIsChartsOpen(true)}
        onOpenAuditLog={() => setIsAuditLogOpen(true)}
        onOpenBenchmarks={() => setIsBenchmarksOpen(true)}
        onNewWorkbook={handleNewWorkbook}
        onSaveWorkbook={handleSaveWorkbook}
        onOpenWorkbook={handleOpenWorkbook}
        onRefreshAll={handleRefreshAll}
        onApplyFormat={handleApplyFormat}
        stagedCount={stagedCount}
      />

      {/* Formula Bar */}
      <FormulaBar
        selectedRow={selectedRow}
        selectedCol={selectedCol}
        activeCellData={activeCellData}
        onCommit={(val) => handleCommitCell(selectedRow, selectedCol, val)}
      />

      {/* Virtualized Worksheet Grid Canvas */}
      <Grid
        sheetName={sheetName}
        viewport={viewport}
        selectedRow={selectedRow}
        selectedCol={selectedCol}
        onSelectCell={(r, c) => {
          setSelectedRow(r);
          setSelectedCol(c);
        }}
        onCommitCell={handleCommitCell}
        onViewportChange={handleViewportChange}
      />

      {/* Bottom Sheet Tabs & Status Bar */}
      <div className="flex items-center justify-between border-t border-slate-300 bg-white px-3 py-1 text-xs select-none shadow-inner">
        {/* Sheet Tabs */}
        <div className="flex items-center gap-1">
          <div className="px-3 py-0.5 bg-slate-100 text-emerald-800 font-bold border-b-2 border-emerald-600 rounded-t text-xs">
            {sheetName}
          </div>
          <button className="px-2 py-0.5 text-slate-400 hover:text-slate-600 font-bold text-xs">
            +
          </button>
        </div>

        {/* Calculation & System Status */}
        <div className="flex items-center gap-4 text-slate-500 text-[11px]">
          <span className="truncate max-w-sm">{statusMsg}</span>
          <span className="font-mono text-slate-400">Gen #{viewport?.generation ?? 0}</span>
          {stagedCount > 0 && (
            <button
              onClick={() => setIsStagedReviewOpen(true)}
              className="text-amber-700 font-bold underline hover:text-amber-800"
            >
              {stagedCount} staged {stagedCount === 1 ? 'edit' : 'edits'} pending writeback
            </button>
          )}
          <span className="font-mono text-slate-400">{activeKey}</span>
        </div>
      </div>

      {/* Modals */}
      <SchemaBrowserModal
        isOpen={isSchemaBrowserOpen}
        onClose={() => setIsSchemaBrowserOpen(false)}
        onOpenQueryWithTable={openQueryWithTable}
      />

      <QueryEditorModal
        isOpen={isQueryEditorOpen}
        onClose={() => setIsQueryEditorOpen(false)}
        initialConnection={queryInitialConn}
        initialSql={queryInitialSql}
        onInsertTableFromSource={(srcName) => {
          api.insertTable(sheetName, srcName, srcName, selectedRow, selectedCol)
            .then(() => {
              refreshMetadata();
              fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
            })
            .catch((err) => alert(err.message));
        }}
      />

      <InsertTableModal
        isOpen={isInsertTableOpen}
        onClose={() => setIsInsertTableOpen(false)}
        activeRow={selectedRow}
        activeCol={selectedCol}
        onTableInserted={() => {
          refreshMetadata();
          fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
        }}
      />

      <StagedReviewModal
        isOpen={isStagedReviewOpen}
        onClose={() => setIsStagedReviewOpen(false)}
        onAppliedSuccessfully={() => {
          refreshMetadata();
          fetchViewportData(visibleBounds.startRow, visibleBounds.startCol, visibleBounds.endRow, visibleBounds.endCol);
        }}
      />

      <PivotBuilderModal
        isOpen={isPivotsOpen}
        onClose={() => setIsPivotsOpen(false)}
        tables={tables}
      />

      <ChartModal
        isOpen={isChartsOpen}
        onClose={() => setIsChartsOpen(false)}
        tables={tables}
      />

      <AuditLogModal
        isOpen={isAuditLogOpen}
        onClose={() => setIsAuditLogOpen(false)}
      />

      <BenchmarkModal
        isOpen={isBenchmarksOpen}
        onClose={() => setIsBenchmarksOpen(false)}
      />
    </div>
  );
};
