import { displayValue } from '../utils/values';
import React, { useState, useEffect } from 'react';
import { X, Play, CheckCircle, AlertCircle, Sparkles, Table as TableIcon, ArrowRight } from 'lucide-react';
import { api } from '../api';
import { QueryResult, LineageReport } from '../types';

interface QueryEditorModalProps {
  isOpen: boolean;
  onClose: () => void;
  initialConnection?: string;
  initialSql?: string;
  onInsertTableFromSource: (sourceName: string) => void;
}

export const QueryEditorModal: React.FC<QueryEditorModalProps> = ({
  isOpen,
  onClose,
  initialConnection,
  initialSql,
  onInsertTableFromSource,
}) => {
  const [connections, setConnections] = useState<{ name: string; kind: string }[]>([]);
  const [selectedConn, setSelectedConn] = useState(initialConnection || '');
  const [sql, setSql] = useState(initialSql || 'SELECT * FROM invoices LIMIT 50');
  const [sourceName, setSourceName] = useState('Invoices');

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<QueryResult | null>(null);
  const [lineage, setLineage] = useState<LineageReport | null>(null);

  useEffect(() => {
    if (isOpen) {
      api.getConnections().then((conns) => {
        setConnections(conns);
        if (conns.length > 0 && !selectedConn) {
          setSelectedConn(conns[0].name);
        }
      });
      if (initialConnection) {
        setSelectedConn(initialConnection);
      }
      if (initialSql) {
        setSql(initialSql);
      }
    }
  }, [isOpen, initialConnection, initialSql]);

  const handleExecute = async () => {
    if (!selectedConn || !sql.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const [qRes, lRes] = await Promise.all([
        api.executeQuery(selectedConn, sql),
        api.analyzeQuery(selectedConn, sql),
      ]);
      setResult(qRes);
      setLineage(lRes);
    } catch (err: any) {
      setError(err.message);
      setResult(null);
      setLineage(null);
    } finally {
      setLoading(false);
    }
  };

  const handleSaveAndInsert = async () => {
    if (!selectedConn || !sql.trim() || !sourceName.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await api.createSource(sourceName, selectedConn, sql);
      onInsertTableFromSource(sourceName);
      onClose();
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-5xl h-[650px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <Sparkles className="w-5 h-5 text-indigo-600" />
            <h2 className="text-sm font-bold text-slate-800">SQL Query Editor & Source Builder</h2>
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
            <span className="font-mono">{error}</span>
            <button onClick={() => setError(null)} className="font-bold underline text-[10px]">
              Dismiss
            </button>
          </div>
        )}

        {/* Content Body */}
        <div className="flex-1 flex flex-col p-4 overflow-hidden gap-3">
          {/* Top Controls: Connection & Presets */}
          <div className="flex items-center justify-between gap-3 text-xs">
            <div className="flex items-center gap-2">
              <label className="font-bold text-slate-700">Target Database:</label>
              <select
                value={selectedConn}
                onChange={(e) => setSelectedConn(e.target.value)}
                className="border border-slate-300 rounded px-2.5 py-1 bg-white font-medium text-slate-800"
              >
                {connections.map((c) => (
                  <option key={c.name} value={c.name}>
                    {c.name} ({c.kind})
                  </option>
                ))}
              </select>
            </div>

            {/* Presets */}
            <div className="flex items-center gap-1.5 text-slate-500">
              <span className="text-[11px] font-semibold">Presets:</span>
              <button
                onClick={() => {
                  setSql('SELECT id, customer_name, amount, status, issued_date FROM invoices');
                  setSourceName('Invoices');
                }}
                className="px-2 py-0.5 bg-slate-100 hover:bg-slate-200 rounded text-[11px] text-slate-700 font-mono"
              >
                Invoices (Editable)
              </button>
              <button
                onClick={() => {
                  setSql('SELECT id, name, quota, region FROM sales_reps');
                  setSourceName('SalesReps');
                }}
                className="px-2 py-0.5 bg-slate-100 hover:bg-slate-200 rounded text-[11px] text-slate-700 font-mono"
              >
                Sales Reps (Editable)
              </button>
              <button
                onClick={() => {
                  setSql('SELECT status, count(*) as count, sum(amount) as total_amount FROM invoices GROUP BY status');
                  setSourceName('InvoiceSummary');
                }}
                className="px-2 py-0.5 bg-slate-100 hover:bg-slate-200 rounded text-[11px] text-slate-700 font-mono"
              >
                Summary (Read-Only)
              </button>
            </div>
          </div>

          {/* SQL Input Area */}
          <div className="h-32 border border-slate-300 rounded overflow-hidden flex flex-col focus-within:border-indigo-500">
            <textarea
              value={sql}
              onChange={(e) => setSql(e.target.value)}
              placeholder="Enter read-only SQL query (e.g. SELECT * FROM invoices)"
              className="flex-1 w-full p-2.5 font-mono text-xs text-slate-800 bg-slate-50/50 resize-none outline-none leading-relaxed"
            />
            <div className="bg-slate-100 px-3 py-1.5 flex items-center justify-between border-t border-slate-200 text-xs">
              <span className="text-slate-500 text-[11px]">
                Read-only capability boundary enforced. Mutating SQL statements are blocked.
              </span>
              <button
                onClick={handleExecute}
                disabled={loading || !sql.trim()}
                className="flex items-center gap-1.5 px-3 py-1 bg-indigo-600 hover:bg-indigo-700 text-white rounded font-semibold transition-colors shadow-sm disabled:opacity-50"
              >
                <Play className="w-3.5 h-3.5 fill-white" />
                <span>{loading ? 'Executing...' : 'Run Query'}</span>
              </button>
            </div>
          </div>

          {/* Lineage & Writeback Status Banner */}
          {lineage && (
            <div
              className={`p-2.5 rounded-md border text-xs flex items-center justify-between ${
                lineage.is_editable
                  ? 'bg-emerald-50 border-emerald-200 text-emerald-900'
                  : 'bg-amber-50 border-amber-200 text-amber-900'
              }`}
            >
              <div className="flex items-center gap-2">
                {lineage.is_editable ? (
                  <CheckCircle className="w-4 h-4 text-emerald-600 flex-shrink-0" />
                ) : (
                  <AlertCircle className="w-4 h-4 text-amber-600 flex-shrink-0" />
                )}
                <div>
                  <div className="font-bold">
                    {lineage.is_editable
                      ? 'Qualified for Safe Staged Writeback'
                      : 'Read-Only Query (Safe View)'}
                  </div>
                  <div className="text-[11px] text-slate-600 mt-0.5">
                    {lineage.is_editable ? (
                      <span>
                        Base table: <span className="font-mono font-semibold">{lineage.base_table}</span> •
                        Identity PK: <span className="font-mono font-semibold">[{lineage.primary_keys.join(', ')}]</span> •
                        Editable columns: {lineage.editable_columns.join(', ')}
                      </span>
                    ) : (
                      <span>
                        {lineage.read_only_reasons.join('; ') || 'Query structure not eligible for in-place row edits.'}
                      </span>
                    )}
                  </div>
                </div>
              </div>

              {lineage.is_editable && (
                <span className="px-2 py-0.5 bg-emerald-600 text-white rounded text-[10px] font-bold uppercase tracking-wider">
                  Editable
                </span>
              )}
            </div>
          )}

          {/* Result Table Preview */}
          <div className="flex-1 border border-slate-200 rounded overflow-auto bg-white">
            {result ? (
              <table className="w-full text-left text-xs border-collapse">
                <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 uppercase text-[10px] sticky top-0">
                  <tr>
                    {result.columns.map((col) => (
                      <th key={col.name} className="px-3 py-2 font-semibold">
                        <div className="flex items-center gap-1">
                          <span>{col.name}</span>
                          {col.is_primary_key && (
                            <span className="text-[9px] bg-amber-100 text-amber-800 px-1 rounded font-bold">
                              PK
                            </span>
                          )}
                        </div>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {result.rows.length === 0 ? (
                    <tr>
                      <td colSpan={result.columns.length} className="px-4 py-8 text-center text-slate-400 italic">
                        Empty result set (0 rows)
                      </td>
                    </tr>
                  ) : (
                    result.rows.slice(0, 100).map((row, rIdx) => (
                      <tr key={rIdx} className="hover:bg-slate-50">
                        {row.map((val, cIdx) => (
                          <td key={cIdx} className="px-3 py-1.5 font-mono text-slate-700 truncate max-w-[200px]">
                            {val === null ? <span className="text-slate-400 italic">NULL</span> : displayValue(val)}
                          </td>
                        ))}
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            ) : (
              <div className="h-full flex flex-col items-center justify-center text-slate-400 text-xs">
                <TableIcon className="w-8 h-8 text-slate-300 mb-2" />
                <span>Execute a query to preview results</span>
              </div>
            )}
          </div>

          {/* Footer: Register as Named Data Source */}
          <div className="pt-2 border-t border-slate-200 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <label className="text-xs font-bold text-slate-700">Source Name:</label>
              <input
                type="text"
                value={sourceName}
                onChange={(e) => setSourceName(e.target.value)}
                placeholder="e.g. Invoices"
                className="px-2.5 py-1 border border-slate-300 rounded text-xs w-48 font-mono bg-white"
              />
            </div>

            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={onClose}
                className="px-3 py-1.5 rounded border border-slate-300 text-xs text-slate-600 hover:bg-slate-100"
              >
                Close
              </button>
              <button
                onClick={handleSaveAndInsert}
                disabled={loading || !result || !sourceName.trim()}
                className="flex items-center gap-1.5 px-4 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded text-xs font-semibold shadow-sm transition-colors disabled:opacity-50"
              >
                <span>Save Source & Insert Table</span>
                <ArrowRight className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
