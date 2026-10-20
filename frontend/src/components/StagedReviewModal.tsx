import React, { useState, useEffect } from 'react';
import { X, CheckCircle, AlertTriangle, Trash2, ArrowRight, ShieldCheck, RefreshCw } from 'lucide-react';
import { displayValue } from '../utils/values';
import { api } from '../api';
import { WritebackReviewItem, AuditRecord } from '../types';

interface StagedReviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAppliedSuccessfully: () => void;
}

export const StagedReviewModal: React.FC<StagedReviewModalProps> = ({
  isOpen,
  onClose,
  onAppliedSuccessfully,
}) => {
  const [reviewItems, setReviewItems] = useState<WritebackReviewItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastAudit, setLastAudit] = useState<AuditRecord | null>(null);

  const loadReview = async () => {
    setLoading(true);
    setError(null);
    try {
      const items = await api.reviewWriteback();
      setReviewItems(items);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      loadReview();
      setLastAudit(null);
    }
  }, [isOpen]);

  const handleDiscard = async (id: string) => {
    try {
      await api.discardStaged(id);
      await loadReview();
      onAppliedSuccessfully();
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleDiscardAll = async () => {
    try {
      await api.discardAllStaged();
      await loadReview();
      onAppliedSuccessfully();
    } catch (err: any) {
      setError(err.message);
    }
  };

  const handleApply = async () => {
    if (reviewItems.length === 0) return;
    const connName = reviewItems[0].patch.connection_name;
    setLoading(true);
    setError(null);
    try {
      const audit = await api.applyWriteback(connName);
      setLastAudit(audit);
      await loadReview();
      onAppliedSuccessfully();
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  const hasConflicts = reviewItems.some((item) => item.conflict_status !== 'NoConflict');
  const nextConnection = reviewItems[0]?.patch.connection_name;
  const allCanApply = reviewItems.length > 0 && reviewItems.filter(i => i.patch.connection_name === nextConnection).every(i => i.can_apply);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-4xl h-[600px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <ShieldCheck className="w-5 h-5 text-amber-600" />
            <h2 className="text-sm font-bold text-slate-800">Review Staged Edits & Database Writeback</h2>
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

        {/* Audit feedback banner */}
        {lastAudit && (
          <div
            className={`px-4 py-2.5 text-xs border-b flex items-center justify-between ${
              lastAudit.status === 'AppliedSuccessfully'
                ? 'bg-emerald-50 border-emerald-200 text-emerald-900'
                : 'bg-rose-50 border-rose-200 text-rose-900'
            }`}
          >
            <div className="flex items-center gap-2">
              <CheckCircle className="w-4 h-4 text-emerald-600" />
              <span>
                <strong>{lastAudit.status}:</strong> {lastAudit.patch_count} patches applied to{' '}
                <span className="font-mono">{lastAudit.table_name}</span> in database{' '}
                <span className="font-mono">{lastAudit.connection_name}</span>.
              </span>
            </div>
            <span className="font-mono text-[10px] text-slate-500">Tx: {lastAudit.id}</span>
          </div>
        )}

        {/* Safety Notice */}
        <div className="bg-amber-50/60 border-b border-amber-200 px-4 py-2 flex items-center justify-between text-xs text-amber-900">
          <div className="flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 text-amber-600 flex-shrink-0" />
            <span>
              <strong>Safe Writeback Protocol:</strong> All edits for one connection are executed in a single atomic database
              transaction using NULL-safe optimistic concurrency. If any row was modified or deleted
              externally, the entire changeset is safely rolled back without data loss.
            </span>
          </div>
          <button
            onClick={loadReview}
            disabled={loading}
            className="flex items-center gap-1 text-slate-600 hover:text-slate-900 text-[11px] font-medium"
          >
            <RefreshCw className="w-3 h-3" />
            <span>Check Conflicts</span>
          </button>
        </div>

        {/* Review Items Table */}
        <div className="flex-1 overflow-auto p-4 bg-white">
          {reviewItems.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-slate-400 text-xs">
              <ShieldCheck className="w-10 h-10 text-slate-300 mb-2" />
              <p className="font-medium text-slate-600">No pending staged edits</p>
              <p className="text-[11px] text-slate-400 mt-1">
                Editing cells in qualified SQL-backed tables queues staged changes here before writing back.
              </p>
            </div>
          ) : (
            <div className="border border-slate-200 rounded overflow-hidden">
              <table className="w-full text-left text-xs border-collapse">
                <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 uppercase text-[10px]">
                  <tr>
                    <th className="px-3 py-2">Connection & Table</th>
                    <th className="px-3 py-2">Primary Key</th>
                    <th className="px-3 py-2">Column</th>
                    <th className="px-3 py-2">Original DB Value</th>
                    <th className="px-3 py-2">Proposed Value</th>
                    <th className="px-3 py-2">Conflict Status</th>
                    <th className="px-3 py-2 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {reviewItems.map((item) => {
                    const { patch, conflict_status } = item;
                    const isConflict = conflict_status !== 'NoConflict';
                    return (
                      <tr
                        key={patch.id}
                        className={`hover:bg-slate-50 ${isConflict ? 'bg-rose-50/50' : ''}`}
                      >
                        <td className="px-3 py-2 font-mono text-slate-700">
                          <span className="font-semibold text-slate-900">{patch.connection_name}</span>.
                          <span>{patch.table_name}</span>
                        </td>
                        <td className="px-3 py-2 font-mono text-slate-600">
                          {patch.pk_columns.map((c, i) => `${c}=${displayValue(patch.pk_values[i])}`).join(', ')}
                        </td>
                        <td className="px-3 py-2 font-mono font-medium text-slate-800">
                          {patch.column_name}
                        </td>
                        <td className="px-3 py-2 font-mono text-slate-500 line-through">
                          {patch.original_value === null ? (
                            <span className="italic">NULL</span>
                          ) : (
                            displayValue(patch.original_value)
                          )}
                        </td>
                        <td className="px-3 py-2 font-mono font-bold text-emerald-700 bg-emerald-50/50">
                          {patch.proposed_value === null ? (
                            <span className="italic">NULL</span>
                          ) : (
                            displayValue(patch.proposed_value)
                          )}
                        </td>
                        <td className="px-3 py-2">
                          {conflict_status === 'NoConflict' ? (
                            <span className="inline-flex items-center gap-1 text-[11px] font-bold text-emerald-700">
                              <CheckCircle className="w-3.5 h-3.5 text-emerald-600" />
                              Ready
                            </span>
                          ) : conflict_status === 'RowDeletedExternally' ? (
                            <span className="inline-flex items-center gap-1 text-[11px] font-bold text-rose-700">
                              <AlertTriangle className="w-3.5 h-3.5 text-rose-600" />
                              Row Deleted Externally
                            </span>
                          ) : (
                            <span className="inline-flex items-center gap-1 text-[11px] font-bold text-rose-700">
                              <AlertTriangle className="w-3.5 h-3.5 text-rose-600" />
                              {'CheckFailed' in conflict_status ? `Check failed: ${conflict_status.CheckFailed.message}` : 'Modified Externally'}
                            </span>
                          )}
                        </td>
                        <td className="px-3 py-2 text-right">
                          <button
                            onClick={() => handleDiscard(patch.id)}
                            title="Discard this edit"
                            className="p-1 text-slate-400 hover:text-rose-600 hover:bg-rose-50 rounded"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Footer Actions */}
        <div className="p-3 border-t border-slate-200 bg-slate-50 flex items-center justify-between text-xs">
          <button
            onClick={handleDiscardAll}
            disabled={reviewItems.length === 0}
            className="flex items-center gap-1.5 px-3 py-1.5 text-slate-600 hover:text-rose-700 hover:bg-rose-50 border border-slate-300 rounded font-medium disabled:opacity-40"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Discard All</span>
          </button>

          <div className="flex items-center gap-2">
            <button
              onClick={onClose}
              className="px-3 py-1.5 rounded border border-slate-300 text-slate-600 hover:bg-slate-200 font-medium"
            >
              Close
            </button>
            <button
              onClick={handleApply}
              disabled={loading || !allCanApply || reviewItems.length === 0}
              className="flex items-center gap-1.5 px-4 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded font-bold shadow-sm transition-colors disabled:opacity-50"
            >
              <span>Apply to {nextConnection ?? 'Database'}</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
