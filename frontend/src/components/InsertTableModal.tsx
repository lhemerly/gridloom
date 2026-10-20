import React, { useState, useEffect } from 'react';
import { X, Table as TableIcon, ArrowRight } from 'lucide-react';
import { api } from '../api';
import { NamedSource } from '../types';
import { coordToA1 } from '../utils/coords';

interface InsertTableModalProps {
  isOpen: boolean;
  onClose: () => void;
  activeRow: number;
  activeCol: number;
  onTableInserted: () => void;
}

export const InsertTableModal: React.FC<InsertTableModalProps> = ({
  isOpen,
  onClose,
  activeRow,
  activeCol,
  onTableInserted,
}) => {
  const [sources, setSources] = useState<NamedSource[]>([]);
  const [selectedSource, setSelectedSource] = useState('');
  const [tableName, setTableName] = useState('');
  const [startRow, setStartRow] = useState(activeRow);
  const [startCol, setStartCol] = useState(activeCol);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setStartRow(activeRow);
      setStartCol(activeCol);
      api.getSources().then((srcs) => {
        setSources(srcs);
        if (srcs.length > 0) {
          setSelectedSource(srcs[0].name);
          setTableName(srcs[0].name);
        }
      });
    }
  }, [isOpen, activeRow, activeCol]);

  const handleSourceChange = (name: string) => {
    setSelectedSource(name);
    setTableName(name);
  };

  const handleInsert = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedSource || !tableName.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await api.insertTable('Sheet1', tableName.trim(), selectedSource, startRow, startCol);
      onTableInserted();
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
      <div className="bg-white rounded-lg shadow-xl w-full max-w-md flex flex-col overflow-hidden border border-slate-200">
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <TableIcon className="w-5 h-5 text-emerald-600" />
            <h2 className="text-sm font-bold text-slate-800">Insert Worksheet Table</h2>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-600 p-1 rounded hover:bg-slate-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {error && (
          <div className="bg-rose-50 text-rose-800 text-xs px-4 py-2 border-b border-rose-200">
            {error}
          </div>
        )}

        <form onSubmit={handleInsert} className="p-4 space-y-3 text-xs">
          <div>
            <label className="block text-[11px] font-bold text-slate-700 mb-1">
              Select Data Source
            </label>
            {sources.length === 0 ? (
              <div className="text-slate-500 italic p-2 border border-slate-200 rounded bg-slate-50">
                No named sources defined yet. Use the SQL Query Editor to create a source first.
              </div>
            ) : (
              <select
                value={selectedSource}
                onChange={(e) => handleSourceChange(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-medium"
              >
                {sources.map((s) => (
                  <option key={s.name} value={s.name}>
                    {s.name} ({s.connection_name})
                  </option>
                ))}
              </select>
            )}
          </div>

          <div>
            <label className="block text-[11px] font-bold text-slate-700 mb-1">
              Table Name (Structured Reference)
            </label>
            <input
              type="text"
              value={tableName}
              onChange={(e) => setTableName(e.target.value)}
              placeholder="e.g. Invoices"
              required
              className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-mono"
            />
            <span className="text-[10px] text-slate-400 mt-0.5 block">
              Reference columns in formulas with e.g. <code className="text-slate-700">{tableName || 'Table'}[Column]</code>
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Anchor Row</label>
              <input
                type="number"
                min={0}
                value={startRow + 1}
                onChange={(e) => setStartRow(Math.max(0, parseInt(e.target.value, 10) - 1))}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-mono"
              />
            </div>
            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Anchor Cell</label>
              <div className="w-full px-2.5 py-1.5 border border-slate-200 bg-slate-50 rounded text-xs font-mono font-bold text-slate-700">
                {coordToA1(startRow, startCol)}
              </div>
            </div>
          </div>

          <div className="pt-3 border-t border-slate-200 flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded border border-slate-300 text-xs text-slate-600 hover:bg-slate-100 font-medium"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || sources.length === 0 || !tableName.trim()}
              className="flex items-center gap-1.5 px-4 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded text-xs font-bold shadow-sm transition-colors disabled:opacity-50"
            >
              <span>Insert Table</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
