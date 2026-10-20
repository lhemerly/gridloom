import { displayValue } from '../utils/values';
import React, { useState, useEffect } from 'react';
import { X, History, CheckCircle, AlertTriangle, Shield } from 'lucide-react';
import { api } from '../api';
import { AuditRecord } from '../types';

interface AuditLogModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AuditLogModal: React.FC<AuditLogModalProps> = ({ isOpen, onClose }) => {
  const [logs, setLogs] = useState<AuditRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setLoading(true);
      setError(null);
      api.getAuditLog()
        .then(setLogs)
        .catch((err) => setError(err.message))
        .finally(() => setLoading(false));
    }
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-4xl h-[560px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <History className="w-5 h-5 text-slate-700" />
            <h2 className="text-sm font-bold text-slate-800">Database Writeback Audit Trail</h2>
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

        {/* Content Body */}
        <div className="flex-1 overflow-auto p-4 bg-white">
          {logs.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-slate-400 text-xs">
              <Shield className="w-10 h-10 text-slate-300 mb-2" />
              <p className="font-medium text-slate-600">No writeback transactions recorded yet</p>
              <p className="text-[11px] text-slate-400 mt-1">
                Every committed or rolled-back staged change set writes a permanent audit record here.
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {logs.map((record) => {
                const isSuccess = record.status === 'AppliedSuccessfully';
                return (
                  <div
                    key={record.id}
                    className="border border-slate-200 rounded-lg p-3 bg-slate-50/50 hover:bg-slate-50 transition-colors"
                  >
                    <div className="flex items-center justify-between pb-2 border-b border-slate-200 text-xs">
                      <div className="flex items-center gap-2">
                        {isSuccess ? (
                          <CheckCircle className="w-4 h-4 text-emerald-600" />
                        ) : (
                          <AlertTriangle className="w-4 h-4 text-rose-600" />
                        )}
                        <span className="font-bold text-slate-800">{record.status}</span>
                        <span className="text-slate-500">•</span>
                        <span className="font-mono text-slate-600">
                          {record.connection_name}.{record.table_name}
                        </span>
                        <span className="text-slate-500">•</span>
                        <span className="font-semibold text-slate-700">
                          {record.patch_count} {record.patch_count === 1 ? 'patch' : 'patches'}
                        </span>
                      </div>
                      <span className="text-[11px] text-slate-500 font-mono">
                        {new Date(record.applied_at).toLocaleString()}
                      </span>
                    </div>

                    {record.error_message && (
                      <div className="mt-2 text-xs text-rose-700 font-mono bg-rose-50 p-2 rounded border border-rose-200">
                        Rollback reason: {record.error_message}
                      </div>
                    )}

                    <div className="mt-2 text-xs">
                      <span className="text-[10px] uppercase font-bold text-slate-400 tracking-wider">
                        Patched Row Keys & Values
                      </span>
                      <div className="flex flex-wrap gap-1.5 mt-1">
                        {record.patches.map((p) => (
                          <span
                            key={p.id}
                            className="inline-flex items-center gap-1 font-mono text-[11px] bg-white border border-slate-300 rounded px-2 py-0.5"
                          >
                            <span className="text-slate-500">
                              {p.pk_columns.map((c, i) => `${c}=${p.pk_values[i]}`).join(',')}
                            </span>
                            <span className="text-slate-400 font-bold">&rarr;</span>
                            <span className="text-slate-800 font-medium">{p.column_name}:</span>
                            <span className="line-through text-slate-400">{displayValue(p.original_value)}</span>
                            <span className="font-bold text-emerald-700">{displayValue(p.proposed_value)}</span>
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-3 border-t border-slate-200 bg-slate-50 flex justify-end">
          <button
            onClick={onClose}
            className="px-3 py-1.5 rounded border border-slate-300 text-xs text-slate-600 hover:bg-slate-200 font-medium"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
