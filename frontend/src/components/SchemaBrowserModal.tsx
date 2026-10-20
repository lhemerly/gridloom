import React, { useState, useEffect } from 'react';
import { X, Database, Plus, RefreshCw, Key, Table as TableIcon, FileText } from 'lucide-react';
import { api } from '../api';
import { TableSchema } from '../types';

interface SchemaBrowserModalProps {
  isOpen: boolean;
  onClose: () => void;
  onOpenQueryWithTable: (connName: string, tableName: string) => void;
}

export const SchemaBrowserModal: React.FC<SchemaBrowserModalProps> = ({
  isOpen,
  onClose,
  onOpenQueryWithTable,
}) => {
  const [connections, setConnections] = useState<{ name: string; kind: string; path: string }[]>([]);
  const [selectedConn, setSelectedConn] = useState<string>('');
  const [tables, setTables] = useState<string[]>([]);
  const [selectedTable, setSelectedTable] = useState<string>('');
  const [schema, setSchema] = useState<TableSchema | null>(null);

  // New connection form
  const [showAddForm, setShowAddForm] = useState(false);
  const [connName, setConnName] = useState('');
  const [connKind, setConnKind] = useState<'sqlite' | 'duckdb'>('sqlite');
  const [connPath, setConnPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadConnections = async () => {
    try {
      setError(null);
      const conns = await api.getConnections();
      setConnections(conns);
      if (conns.length > 0 && !selectedConn) {
        setSelectedConn(conns[0].name);
      }
    } catch (err: any) {
      setError(err.message);
    }
  };

  useEffect(() => {
    if (isOpen) {
      loadConnections();
    }
  }, [isOpen]);

  // Load tables whenever selectedConn changes
  useEffect(() => {
    if (!selectedConn) {
      setTables([]);
      return;
    }
    api.getTables(selectedConn)
      .then((tbls) => {
        setTables(tbls);
        if (tbls.length > 0) {
          setSelectedTable(tbls[0]);
        } else {
          setSelectedTable('');
          setSchema(null);
        }
      })
      .catch((err) => setError(err.message));
  }, [selectedConn]);

  // Load table schema whenever selectedTable changes
  useEffect(() => {
    if (!selectedConn || !selectedTable) {
      setSchema(null);
      return;
    }
    api.getTableSchema(selectedConn, selectedTable)
      .then(setSchema)
      .catch((err) => setError(err.message));
  }, [selectedConn, selectedTable]);

  const handleCreateConnection = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!connName || !connPath) return;
    setLoading(true);
    setError(null);
    try {
      await api.connectDb(connName, connKind, connPath);
      await loadConnections();
      setSelectedConn(connName);
      setShowAddForm(false);
      setConnName('');
      setConnPath('');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleConnectSample = async (kind: 'sqlite' | 'duckdb') => {
    setLoading(true);
    setError(null);
    try {
      const name = kind === 'sqlite' ? 'SampleSales' : 'SampleAnalytics';
      const path = kind === 'sqlite' ? 'fixtures/sample_sales.sqlite' : 'fixtures/sample_analytics.duckdb';
      await api.connectDb(name, kind, path);
      await loadConnections();
      setSelectedConn(name);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-4xl h-[580px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <Database className="w-5 h-5 text-blue-600" />
            <h2 className="text-sm font-bold text-slate-800">Database Schema Browser</h2>
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
          {/* Left Panel: Connections & Tables */}
          <div className="w-1/3 border-r border-slate-200 flex flex-col bg-slate-50/50">
            {/* Connections Section */}
            <div className="p-3 border-b border-slate-200">
              <div className="flex items-center justify-between mb-2">
                <span className="text-[11px] font-bold text-slate-500 uppercase tracking-wider">
                  Connections
                </span>
                <button
                  onClick={() => setShowAddForm(!showAddForm)}
                  className="flex items-center gap-1 text-xs text-blue-600 hover:text-blue-800 font-semibold"
                >
                  <Plus className="w-3 h-3" />
                  <span>Connect</span>
                </button>
              </div>

              {/* Sample Buttons */}
              <div className="flex gap-1.5 mb-2">
                <button
                  onClick={() => handleConnectSample('sqlite')}
                  disabled={loading}
                  className="flex-1 py-1 px-1.5 bg-white border border-slate-300 rounded text-[10px] font-medium text-slate-700 hover:bg-slate-100 shadow-sm"
                >
                  + Sample SQLite
                </button>
                <button
                  onClick={() => handleConnectSample('duckdb')}
                  disabled={loading}
                  className="flex-1 py-1 px-1.5 bg-white border border-slate-300 rounded text-[10px] font-medium text-slate-700 hover:bg-slate-100 shadow-sm"
                >
                  + Sample DuckDB
                </button>
              </div>

              {/* Connections dropdown */}
              <select
                value={selectedConn}
                onChange={(e) => setSelectedConn(e.target.value)}
                className="w-full text-xs border border-slate-300 rounded px-2 py-1.5 bg-white font-medium text-slate-800"
              >
                {connections.length === 0 && <option value="">No connections</option>}
                {connections.map((c) => (
                  <option key={c.name} value={c.name}>
                    {c.name} ({c.kind})
                  </option>
                ))}
              </select>
            </div>

            {/* Add Connection Form Drawer */}
            {showAddForm && (
              <form onSubmit={handleCreateConnection} className="p-3 bg-blue-50/40 border-b border-blue-200 space-y-2 text-xs">
                <div>
                  <label className="block text-[10px] font-semibold text-slate-600">Connection Name</label>
                  <input
                    type="text"
                    value={connName}
                    onChange={(e) => setConnName(e.target.value)}
                    placeholder="e.g. SalesDB"
                    required
                    className="w-full px-2 py-1 border border-slate-300 rounded text-xs bg-white"
                  />
                </div>
                <div>
                  <label className="block text-[10px] font-semibold text-slate-600">Database Kind</label>
                  <select
                    value={connKind}
                    onChange={(e) => setConnKind(e.target.value as any)}
                    className="w-full px-2 py-1 border border-slate-300 rounded text-xs bg-white"
                  >
                    <option value="sqlite">SQLite</option>
                    <option value="duckdb">DuckDB</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-semibold text-slate-600">File Path</label>
                  <input
                    type="text"
                    value={connPath}
                    onChange={(e) => setConnPath(e.target.value)}
                    placeholder="/path/to/database.db"
                    required
                    className="w-full px-2 py-1 border border-slate-300 rounded text-xs bg-white"
                  />
                </div>
                <div className="flex justify-end gap-1.5 pt-1">
                  <button
                    type="button"
                    onClick={() => setShowAddForm(false)}
                    className="px-2 py-1 rounded border border-slate-300 text-[10px] text-slate-600 hover:bg-slate-100"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={loading}
                    className="px-2.5 py-1 rounded bg-blue-600 text-white text-[10px] font-semibold hover:bg-blue-700"
                  >
                    Connect
                  </button>
                </div>
              </form>
            )}

            {/* Tables List */}
            <div className="flex-1 overflow-y-auto p-2">
              <span className="text-[10px] font-bold text-slate-400 uppercase tracking-wider px-2 block mb-1">
                Tables & Views
              </span>
              {tables.length === 0 ? (
                <div className="text-xs text-slate-400 italic px-2 py-4">No tables found</div>
              ) : (
                <div className="space-y-0.5">
                  {tables.map((t) => (
                    <button
                      key={t}
                      onClick={() => setSelectedTable(t)}
                      className={`w-full text-left px-2.5 py-1.5 rounded text-xs flex items-center gap-2 transition-colors ${
                        selectedTable === t
                          ? 'bg-blue-100 text-blue-900 font-semibold'
                          : 'hover:bg-slate-100 text-slate-700'
                      }`}
                    >
                      <TableIcon className="w-3.5 h-3.5 text-slate-400" />
                      <span className="truncate">{t}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Right Panel: Schema Columns & Details */}
          <div className="flex-1 flex flex-col bg-white">
            {selectedTable && schema ? (
              <div className="flex-1 flex flex-col p-4 overflow-hidden">
                <div className="flex items-center justify-between pb-3 border-b border-slate-200">
                  <div>
                    <h3 className="text-sm font-bold text-slate-800 flex items-center gap-2">
                      <TableIcon className="w-4 h-4 text-emerald-600" />
                      <span>{schema.table_name}</span>
                    </h3>
                    <p className="text-xs text-slate-500 mt-0.5">
                      {schema.columns.length} columns • Primary Keys:{' '}
                      {schema.primary_keys.length > 0 ? (
                        <span className="font-mono text-emerald-700 font-semibold">
                          {schema.primary_keys.join(', ')}
                        </span>
                      ) : (
                        <span className="italic text-slate-400">None declared</span>
                      )}
                    </p>
                  </div>
                  <button
                    onClick={() => {
                      onOpenQueryWithTable(selectedConn, schema.table_name);
                      onClose();
                    }}
                    className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded text-xs font-semibold shadow-sm transition-colors"
                  >
                    <FileText className="w-3.5 h-3.5" />
                    <span>Query Table</span>
                  </button>
                </div>

                {/* Columns Table */}
                <div className="flex-1 overflow-auto mt-3 border border-slate-200 rounded">
                  <table className="w-full text-left text-xs">
                    <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 font-semibold uppercase text-[10px]">
                      <tr>
                        <th className="px-3 py-2">Column</th>
                        <th className="px-3 py-2">Type</th>
                        <th className="px-3 py-2">Primary Key</th>
                        <th className="px-3 py-2">Nullable</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-100">
                      {schema.columns.map((col) => (
                        <tr key={col.name} className="hover:bg-slate-50/50">
                          <td className="px-3 py-2 font-mono font-medium text-slate-800">
                            {col.name}
                          </td>
                          <td className="px-3 py-2 text-slate-500 font-mono">
                            {col.data_type}
                          </td>
                          <td className="px-3 py-2">
                            {col.is_primary_key ? (
                              <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-100 text-amber-800 border border-amber-200">
                                <Key className="w-2.5 h-2.5 text-amber-600" />
                                PK
                              </span>
                            ) : (
                              <span className="text-slate-300">-</span>
                            )}
                          </td>
                          <td className="px-3 py-2 text-slate-500">
                            {col.is_nullable ? 'Yes' : 'No'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center text-slate-400 text-xs">
                <Database className="w-8 h-8 text-slate-300 mb-2" />
                <span>Select a connection and table to view schema</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
