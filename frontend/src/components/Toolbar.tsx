import React from 'react';
import {
  Database,
  Terminal,
  Table as TableIcon,
  CheckCircle2,
  FileSpreadsheet,
  Save,
  FolderOpen,
  PieChart,
  BarChart2,
  History,
  Zap,
  RefreshCw,
  DollarSign,
  Percent,
} from 'lucide-react';

interface ToolbarProps {
  onOpenSchemaBrowser: () => void;
  onOpenQueryEditor: () => void;
  onOpenInsertTable: () => void;
  onOpenStagedReview: () => void;
  onOpenPivots: () => void;
  onOpenCharts: () => void;
  onOpenAuditLog: () => void;
  onOpenBenchmarks: () => void;
  onNewWorkbook: () => void;
  onSaveWorkbook: () => void;
  onOpenWorkbook: () => void;
  onRefreshAll: () => void;
  onApplyFormat: (format: string) => void;
  stagedCount: number;
}

export const Toolbar: React.FC<ToolbarProps> = ({
  onOpenSchemaBrowser,
  onOpenQueryEditor,
  onOpenInsertTable,
  onOpenStagedReview,
  onOpenPivots,
  onOpenCharts,
  onOpenAuditLog,
  onOpenBenchmarks,
  onNewWorkbook,
  onSaveWorkbook,
  onOpenWorkbook,
  onRefreshAll,
  onApplyFormat,
  stagedCount,
}) => {
  return (
    <div className="flex flex-wrap items-center justify-between border-b border-slate-200 bg-white px-3 py-1.5 shadow-sm text-xs select-none">
      {/* Brand & File Operations */}
      <div className="flex items-center gap-1.5 border-r border-slate-200 pr-3">
        <div className="flex items-center gap-1 font-bold text-slate-800 text-sm tracking-tight mr-2">
          <div className="w-5 h-5 rounded bg-emerald-600 flex items-center justify-center text-white text-xs font-black shadow-sm">
            G
          </div>
          <span>Gridloom</span>
          <span className="text-[10px] px-1 py-0.2 bg-emerald-100 text-emerald-800 rounded font-semibold ml-0.5">
            MVP
          </span>
        </div>

        <button
          onClick={onNewWorkbook}
          title="New Workbook"
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-700 transition-colors"
        >
          <FileSpreadsheet className="w-3.5 h-3.5 text-slate-500" />
          <span>New</span>
        </button>

        <button
          onClick={onOpenWorkbook}
          title="Open Workbook (.gridloom)"
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-700 transition-colors"
        >
          <FolderOpen className="w-3.5 h-3.5 text-slate-500" />
          <span>Open</span>
        </button>

        <button
          onClick={onSaveWorkbook}
          title="Save Workbook"
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-700 transition-colors"
        >
          <Save className="w-3.5 h-3.5 text-slate-500" />
          <span>Save</span>
        </button>
      </div>

      {/* SQL & Database Controls */}
      <div className="flex items-center gap-1 border-r border-slate-200 px-3">
        <button
          onClick={onOpenSchemaBrowser}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-slate-100 hover:bg-slate-200 text-slate-800 font-medium transition-colors"
        >
          <Database className="w-3.5 h-3.5 text-blue-600" />
          <span>Databases</span>
        </button>

        <button
          onClick={onOpenQueryEditor}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-slate-100 hover:bg-slate-200 text-slate-800 font-medium transition-colors"
        >
          <Terminal className="w-3.5 h-3.5 text-indigo-600" />
          <span>SQL Query</span>
        </button>

        <button
          onClick={onOpenInsertTable}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-slate-100 hover:bg-slate-200 text-slate-800 font-medium transition-colors"
        >
          <TableIcon className="w-3.5 h-3.5 text-emerald-600" />
          <span>Insert Table</span>
        </button>

        <button
          onClick={onRefreshAll}
          title="Refresh All Data Sources"
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-600 transition-colors"
        >
          <RefreshCw className="w-3.5 h-3.5" />
          <span>Refresh</span>
        </button>
      </div>

      {/* Cell Formatting */}
      <div className="flex items-center gap-1 border-r border-slate-200 px-3">
        <button
          onClick={() => onApplyFormat('General')}
          title="Format as General"
          className="px-2 py-1 rounded hover:bg-slate-100 text-slate-700 font-mono"
        >
          123
        </button>
        <button
          onClick={() => onApplyFormat('Currency')}
          title="Format as Currency ($)"
          className="p-1 rounded hover:bg-slate-100 text-slate-700"
        >
          <DollarSign className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={() => onApplyFormat('Percent')}
          title="Format as Percent (%)"
          className="p-1 rounded hover:bg-slate-100 text-slate-700"
        >
          <Percent className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Staged Writeback Badge & Review */}
      <div className="flex items-center gap-1 border-r border-slate-200 px-3">
        <button
          onClick={onOpenStagedReview}
          className={`flex items-center gap-1.5 px-2.5 py-1 rounded font-semibold transition-all shadow-sm ${
            stagedCount > 0
              ? 'bg-amber-100 text-amber-900 border border-amber-300 hover:bg-amber-200 animate-pulse'
              : 'bg-slate-100 text-slate-600 hover:bg-slate-200'
          }`}
        >
          <CheckCircle2
            className={`w-3.5 h-3.5 ${
              stagedCount > 0 ? 'text-amber-700' : 'text-slate-400'
            }`}
          />
          <span>Review Edits</span>
          <span
            className={`px-1.5 py-0.2 rounded-full text-[10px] font-bold ${
              stagedCount > 0 ? 'bg-amber-500 text-white' : 'bg-slate-300 text-slate-700'
            }`}
          >
            {stagedCount}
          </span>
        </button>
      </div>

      {/* Analysis, Charts, Benchmarks & Audit */}
      <div className="flex items-center gap-1 pl-2">
        <button
          onClick={onOpenPivots}
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-700 transition-colors"
        >
          <PieChart className="w-3.5 h-3.5 text-purple-600" />
          <span>Pivot</span>
        </button>

        <button
          onClick={onOpenCharts}
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-700 transition-colors"
        >
          <BarChart2 className="w-3.5 h-3.5 text-blue-600" />
          <span>Charts</span>
        </button>

        <button
          onClick={onOpenAuditLog}
          title="Audit Log of Applied Writebacks"
          className="flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 text-slate-700 transition-colors"
        >
          <History className="w-3.5 h-3.5 text-slate-600" />
          <span>Audit</span>
        </button>

        <button
          onClick={onOpenBenchmarks}
          title="Performance Acceptance Protocol"
          className="flex items-center gap-1 px-2.5 py-1 rounded bg-emerald-50 text-emerald-800 hover:bg-emerald-100 font-semibold border border-emerald-200 transition-colors"
        >
          <Zap className="w-3.5 h-3.5 text-emerald-600" />
          <span>Benchmarks</span>
        </button>
      </div>
    </div>
  );
};
