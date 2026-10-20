import React, { useState } from 'react';
import { X, Zap, CheckCircle2, Play, Activity } from 'lucide-react';
import { api } from '../api';
import { BenchmarkSuiteReport } from '../types';

interface BenchmarkModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const BenchmarkModal: React.FC<BenchmarkModalProps> = ({ isOpen, onClose }) => {
  const [report, setReport] = useState<BenchmarkSuiteReport | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRun = async () => {
    setRunning(true);
    setError(null);
    try {
      const res = await api.runBenchmarks();
      setReport(res);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setRunning(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-4xl h-[600px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <Zap className="w-5 h-5 text-emerald-600" />
            <h2 className="text-sm font-bold text-slate-800">
              V1 Performance Acceptance Protocol Benchmark Suite
            </h2>
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
        <div className="flex-1 flex flex-col p-4 bg-white overflow-hidden gap-3">
          <div className="bg-emerald-50/50 border border-emerald-200 rounded p-3 text-xs text-emerald-900 flex items-center justify-between">
            <div>
              <span className="font-bold">Standardized Automated Benchmark Suite:</span>
              <p className="text-[11px] text-emerald-800 mt-0.5">
                Measures dependency recalculation, 100k-row DuckDB analytical scan, vectorized XLOOKUP,
                and viewport slicing. Resource smoke-test budgets: 30 seconds per workload and 1 GiB peak process RSS (Linux).
              </p>
            </div>
            <button
              onClick={handleRun}
              disabled={running}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded text-xs font-bold shadow-sm transition-colors disabled:opacity-50"
            >
              {running ? (
                <>
                  <Activity className="w-3.5 h-3.5 animate-spin" />
                  <span>Benchmarking...</span>
                </>
              ) : (
                <>
                  <Play className="w-3.5 h-3.5 fill-white" />
                  <span>Run Benchmarks</span>
                </>
              )}
            </button>
          </div>

          {/* Results Table */}
          <div className="flex-1 border border-slate-200 rounded overflow-auto">
            {report ? (
              <table className="w-full text-left text-xs border-collapse">
                <thead className="bg-slate-50 border-b border-slate-200 text-slate-600 uppercase text-[10px]">
                  <tr>
                    <th className="px-3 py-2">Workload</th>
                    <th className="px-3 py-2 text-right">Operations</th>
                    <th className="px-3 py-2 text-right">Elapsed</th>
                    <th className="px-3 py-2 text-right">Throughput</th>
                    <th className="px-3 py-2 text-right">Peak process RSS</th>
                    <th className="px-3 py-2 text-center">Status</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 font-mono">
                  {report.results.map((r, i) => (
                    <tr key={i} className="hover:bg-slate-50">
                      <td className="px-3 py-2 font-sans font-medium text-slate-800">
                        <div>{r.workload_name}</div>
                        <div className="text-[10px] text-slate-400 font-sans">{r.notes}</div>
                      </td>
                      <td className="px-3 py-2 text-right text-slate-600">
                        {r.operations.toLocaleString()}
                      </td>
                      <td className="px-3 py-2 text-right text-slate-600">
                        {r.elapsed_ms.toFixed(2)} ms
                      </td>
                      <td className="px-3 py-2 text-right font-bold text-emerald-700">
                        {Math.round(r.throughput_ops_per_sec).toLocaleString()} ops/s
                      </td>
                      <td className="px-3 py-2 text-right text-slate-500">
                        {r.memory_peak_rss_kb?.toLocaleString() ?? 'Unmeasured'} KB
                      </td>
                      <td className="px-3 py-2 text-center">
                        <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-bold bg-emerald-100 text-emerald-800">
                          <CheckCircle2 className="w-3 h-3 text-emerald-600" />
                          {r.status}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : (
              <div className="h-full flex flex-col items-center justify-center text-slate-400 text-xs">
                <Zap className="w-8 h-8 text-slate-300 mb-2" />
                <span>Click &quot;Run Benchmarks&quot; to execute the test harness</span>
              </div>
            )}
          </div>

          {/* Gate Verification Footer */}
          {report && (
            <div className="flex items-center justify-between p-3 bg-slate-50 border border-slate-200 rounded text-xs">
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-1.5">
                  <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                  <span className="font-semibold text-slate-700">
                    Correctness Gate:{' '}
                    <span className="text-emerald-700">
                      {report.passed_correctness_gate ? 'PASSED' : 'FAILED'}
                    </span>
                  </span>
                </div>
                <div className="flex items-center gap-1.5">
                  <CheckCircle2 className="w-4 h-4 text-emerald-600" />
                  <span className="font-semibold text-slate-700">
                    Resource Gate:{' '}
                    <span className="text-emerald-700">
                      {report.passed_resource_gate === null ? 'UNMEASURED' : report.passed_resource_gate ? 'PASSED' : 'FAILED'}
                    </span>
                  </span>
                </div>
              </div>
              <span className="font-mono text-slate-500 text-[11px]">
                Total: {report.total_elapsed_ms.toFixed(2)} ms across {report.total_workloads} workloads
              </span>
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
