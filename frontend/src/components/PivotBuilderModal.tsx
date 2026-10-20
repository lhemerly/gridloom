import React, { useState, useEffect } from 'react';
import { X, PieChart, Play, Table as TableIcon } from 'lucide-react';
import { api } from '../api';
import { PivotAggregation, PivotDefinition, PivotResult, WorksheetTable } from '../types';

interface PivotBuilderModalProps {
  isOpen: boolean;
  onClose: () => void;
  tables: WorksheetTable[];
}

export const PivotBuilderModal: React.FC<PivotBuilderModalProps> = ({
  isOpen,
  onClose,
  tables,
}) => {
  const [pivotName, setPivotName] = useState('SalesPivot');
  const [sourceTable, setSourceTable] = useState('');
  const [availableColumns, setAvailableColumns] = useState<string[]>([]);
  const [rowField, setRowField] = useState('');
  const [colField, setColField] = useState('');
  const [valField, setValField] = useState('');
  const [aggregation, setAggregation] = useState<PivotAggregation>('Sum');

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pivotResult, setPivotResult] = useState<PivotResult | null>(null);

  useEffect(() => {
    if (tables.length > 0 && !sourceTable) {
      setSourceTable(tables[0].name);
      setAvailableColumns(tables[0].columns);
      if (tables[0].columns.length > 0) {
        setRowField(tables[0].columns[0]);
        setValField(tables[0].columns[tables[0].columns.length - 1]);
      }
    }
  }, [tables, sourceTable]);

  const handleTableChange = (tName: string) => {
    setSourceTable(tName);
    const tbl = tables.find((t) => t.name === tName);
    if (tbl) {
      setAvailableColumns(tbl.columns);
      setRowField(tbl.columns[0] || '');
      setValField(tbl.columns[tbl.columns.length - 1] || '');
    }
  };

  const handleCompute = async () => {
    if (!sourceTable || !rowField || !valField) return;
    setLoading(true);
    setError(null);
    try {
      const def: PivotDefinition = {
        name: pivotName,
        source_table: sourceTable,
        row_fields: [rowField],
        col_fields: colField ? [colField] : [],
        value_field: valField,
        aggregation,
      };
      await api.savePivot(def);
      const res = await api.getPivotData(pivotName);
      setPivotResult(res);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-4xl h-[560px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <PieChart className="w-5 h-5 text-purple-600" />
            <h2 className="text-sm font-bold text-slate-800">Pivot Table Builder</h2>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-600 p-1 rounded hover:bg-slate-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {error && (
          <div className="bg-rose-50 text-rose-800 text-xs px-4 py-2 border-b border-rose-200 flex justify-between items-center">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="font-bold underline text-[10px]">
              Dismiss
            </button>
          </div>
        )}

        {/* Content Body */}
        <div className="flex-1 flex overflow-hidden">
          {/* Left Config Panel */}
          <div className="w-80 border-r border-slate-200 p-4 bg-slate-50/50 flex flex-col gap-3 text-xs">
            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Pivot Name</label>
              <input
                type="text"
                value={pivotName}
                onChange={(e) => setPivotName(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              />
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Source Table</label>
              <select
                value={sourceTable}
                onChange={(e) => handleTableChange(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-medium"
              >
                {tables.map((t) => (
                  <option key={t.name} value={t.name}>
                    {t.name} ({t.columns.length} cols)
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Row Dimension</label>
              <select
                value={rowField}
                onChange={(e) => setRowField(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              >
                {availableColumns.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Column Dimension (Optional)</label>
              <select
                value={colField}
                onChange={(e) => setColField(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              >
                <option value="">(None)</option>
                {availableColumns.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Value Field</label>
              <select
                value={valField}
                onChange={(e) => setValField(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              >
                {availableColumns.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Aggregation Function</label>
              <select
                value={aggregation}
                onChange={(e) => setAggregation(e.target.value as PivotAggregation)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-semibold text-slate-800"
              >
                <option value="Sum">Sum</option>
                <option value="Count">Count</option>
                <option value="Average">Average</option>
                <option value="Min">Minimum</option>
                <option value="Max">Maximum</option>
              </select>
            </div>

            <div className="pt-2 mt-auto">
              <button
                onClick={handleCompute}
                disabled={loading || !sourceTable || !rowField || !valField}
                className="w-full flex items-center justify-center gap-1.5 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded font-bold shadow-sm transition-colors disabled:opacity-50"
              >
                <Play className="w-3.5 h-3.5 fill-white" />
                <span>{loading ? 'Computing...' : 'Generate Pivot Table'}</span>
              </button>
            </div>
          </div>

          {/* Right Results Panel */}
          <div className="flex-1 p-4 bg-white overflow-auto flex flex-col">
            {pivotResult ? (
              <div className="flex-1 border border-slate-200 rounded overflow-auto">
                <table className="w-full text-left text-xs border-collapse">
                  <thead className="bg-purple-50 text-purple-900 border-b border-purple-200 sticky top-0">
                    <tr>
                      {pivotResult.headers.map((h, i) => (
                        <th key={i} className="px-3 py-2 font-bold uppercase text-[10px]">
                          {h}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-100">
                    {pivotResult.rows.map((row, rIdx) => (
                      <tr key={rIdx} className="hover:bg-slate-50">
                        {row.map((val, cIdx) => (
                          <td
                            key={cIdx}
                            className={`px-3 py-1.5 font-mono text-slate-700 ${
                              cIdx > 0 ? 'text-right' : 'font-medium'
                            }`}
                          >
                            {val === null ? (
                              <span className="text-slate-300 italic">0</span>
                            ) : typeof val === 'number' ? (
                              val.toLocaleString(undefined, { maximumFractionDigits: 2 })
                            ) : (
                              String(val)
                            )}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center text-slate-400 text-xs">
                <TableIcon className="w-8 h-8 text-slate-300 mb-2" />
                <span>Configure pivot dimensions and generate the summary table</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
